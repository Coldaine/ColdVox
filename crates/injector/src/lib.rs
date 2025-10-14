use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;

use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop};
use tokio::time::timeout;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;
use zbus::Connection;

/// How to try injection.
#[derive(Clone, Copy, Debug)]
pub enum Mode {
    /// Try portal first, then fall back to uinput (ydotool).
    Auto,
    /// Only use the Wayland portal; never fall back.
    PortalOnly,
    /// Only use uinput/ydotool; never attempt portal.
    UinputOnly,
}

/// Tunables for retries/timeouts and behavior.
#[derive(Clone, Debug)]
pub struct InjectOptions {
    pub mode: Mode,
    /// Hard deadline for the *portal* path (create session + user grant).
    pub portal_deadline: Duration,   // e.g., 5–10s
    /// If user denies portal permission, should we fall back?
    pub fallback_on_denied: bool,               // usually true
}

impl Default for InjectOptions {
    fn default() -> Self {
        Self {
            mode: Mode::Auto,
            portal_deadline: Duration::from_secs(8),
            fallback_on_denied: true,
        }
    }
}

/// Which backend actually injected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendUsed {
    Portal,
    Uinput,
}

/// Success metadata.
#[derive(Clone, Debug)]
pub struct InjectReport {
    pub backend: BackendUsed,
    pub chars_sent: usize,
    pub elapsed: Duration,
}

/// Error taxonomy you can pattern-match on.
#[derive(Error, Debug)]
pub enum InjectError {
    // Portal path
    #[error("portal unavailable (service not present or feature unsupported)")]
    PortalUnavailable,
    #[error("portal permission denied by user")]
    PortalDenied,
    #[error("portal timed out while creating/starting session")]
    PortalTimeout,
    #[error("portal runtime error: {0}")]
    PortalOther(String),

    // uinput/ydotool path
    #[error("ydotool not found or not runnable")]
    YdotoolNotFound,
    #[error("ydotool daemon/socket not reachable (/tmp/.ydotool_socket)")]
    YdotoolSocketUnavailable,
    #[error("uinput permissions (needs input group)")]
    UinputPermission,

    // Generic
    #[error("no backend could inject (see logs)")]
    Exhausted,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// One-shot injection that orchestrates portal + fallback.
pub async fn inject_text(text: &str, opts: &InjectOptions) -> Result<InjectReport, InjectError> {
    match opts.mode {
        Mode::PortalOnly => return try_portal(text, opts.portal_deadline).await,
        Mode::UinputOnly => return try_ydotool(text).await,
        Mode::Auto => {}
    }

    // AUTO flow
    let portal_available = portal_service_present().await;
    if portal_available {
        match try_portal(text, opts.portal_deadline).await {
            Ok(r) => return Ok(r),
            Err(InjectError::PortalDenied) if !opts.fallback_on_denied => {
                return Err(InjectError::PortalDenied)
            }
            Err(_) => { /* fall through to ydotool */ }
        }
    }

    // Fallback
    match try_ydotool(text).await {
        Ok(r) => Ok(r),
        Err(e) => {
            // If we never tried portal (service missing) and ydotool failed, surface a composite
            if !portal_available {
                return Err(e);
            }
            Err(match e {
                InjectError::YdotoolNotFound
                | InjectError::YdotoolSocketUnavailable
                | InjectError::UinputPermission => e,
                _ => InjectError::Exhausted,
            })
        }
    }
}


async fn portal_service_present() -> bool {
    if let Ok(conn) = Connection::session().await {
        if let Ok(dbus) = DBusProxy::new(&conn).await {
            if let Ok(name) = BusName::try_from("org.freedesktop.portal.Desktop") {
                return dbus.name_has_owner(name).await.unwrap_or(false);
            }
        }
    }
    false
}

fn map_char_to_keysym(c: char) -> u32 {
    // This is a simplification. A real implementation would need a proper
    // XKB-style keysym mapping. For many characters, the Unicode codepoint
    // is a valid keysym.
    c as u32
}

async fn try_portal(text: &str, deadline: Duration) -> Result<InjectReport, InjectError> {
    let fut = async {
        let proxy = RemoteDesktop::new()
            .await
            .map_err(|_| InjectError::PortalUnavailable)?;

        let session = proxy
            .create_session()
            .await
            .map_err(|e| InjectError::PortalOther(e.to_string()))?;

        // Start & request keyboard device (triggers auth)
        let req = proxy
            .start(&session, None)
            .await
            .map_err(|e| InjectError::PortalOther(e.to_string()))?;
        let resp = req.response().map_err(|_| InjectError::PortalDenied)?; // maps denial/closed into a clean variant

        if !resp.devices().contains(DeviceType::Keyboard) {
            return Err(InjectError::PortalDenied);
        }

        // Send text (prefer keysym for Unicode text)
        for ch in text.chars() {
            let keysym = map_char_to_keysym(ch); // implement or use a table
            proxy
                .notify_keyboard_keysym(&session, keysym as i32, KeyState::Pressed)
                .await
                .map_err(|e| InjectError::PortalOther(e.to_string()))?;
            proxy
                .notify_keyboard_keysym(&session, keysym as i32, KeyState::Released)
                .await
                .map_err(|e| InjectError::PortalOther(e.to_string()))?;
        }

        Ok::<_, InjectError>(())
    };

    let started = Instant::now();
    timeout(deadline, fut)
        .await
        .map_err(|_| InjectError::PortalTimeout)??;

    Ok(InjectReport {
        backend: BackendUsed::Portal,
        chars_sent: text.chars().count(),
        elapsed: started.elapsed(),
    })
}


async fn try_ydotool(text: &str) -> Result<InjectReport, InjectError> {
    let started = Instant::now();

    // Check presence
    let which = Command::new("which").arg("ydotool").output().map_err(|e| InjectError::Io(e))?;
    if !which.status.success() {
        return Err(InjectError::YdotoolNotFound);
    }

    // Spawn `ydotool type --file -` and write to stdin
    let mut child = Command::new("ydotool")
        .args(["type", "--file", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                InjectError::UinputPermission
            } else {
                InjectError::Io(e)
            }
        })?;

    {
        let stdin = child.stdin.as_mut().ok_or(InjectError::YdotoolSocketUnavailable)?;
        stdin.write_all(text.as_bytes()).map_err(InjectError::Io)?;
    }

    let output = child.wait_with_output().map_err(InjectError::Io)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("could not connect to ydotoold") {
            return Err(InjectError::YdotoolSocketUnavailable);
        }
        return Err(InjectError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("ydotool failed: {}", stderr),
        )));
    }

    Ok(InjectReport {
        backend: BackendUsed::Uinput,
        chars_sent: text.chars().count(),
        elapsed: started.elapsed(),
    })
}