#![cfg(feature = "text-injection-portal-eis")]

use crate::text_injection::{InjectionError, TextInjector};
use crate::text_injection::types::InjectionMetrics;
use std::env;
use async_trait::async_trait;
use ashpd::desktop::{
    remote_desktop::{DeviceType, KeyState, RemoteDesktop},
    PersistMode,
};
use arboard::Clipboard;

#[derive(Debug)]
pub struct PortalEisInjector {
    metrics: InjectionMetrics,
}

impl PortalEisInjector {
    pub fn new() -> Self {
        Self::default()
    }

    fn on_desktop_with_portal() -> bool {
        (env::var_os("WAYLAND_DISPLAY").is_some() || env::var_os("DISPLAY").is_some())
            && env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
    }
}

impl Default for PortalEisInjector {
    fn default() -> Self {
        Self {
            metrics: InjectionMetrics::default(),
        }
    }
}

#[async_trait]
impl TextInjector for PortalEisInjector {
    fn name(&self) -> &'static str {
        "XDG-Portal-EIS"
    }

    fn is_available(&self) -> bool {
        Self::on_desktop_with_portal()
    }

    async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        // 1. Set clipboard text
        let mut clipboard = Clipboard::new().map_err(|e| {
            InjectionError::PermissionDenied(format!("Failed to initialize clipboard: {e}"))
        })?;
        clipboard.set_text(text.to_string()).map_err(|e| {
            InjectionError::PermissionDenied(format!("Failed to set clipboard text: {e}"))
        })?;

        // 2. Use ashpd to request remote control and paste
        let proxy = RemoteDesktop::new().await.map_err(|e| {
            InjectionError::MethodNotAvailable(format!("Failed to connect to remote desktop portal: {e}"))
        })?;

        let session = proxy.create_session().await.map_err(|e| {
            InjectionError::MethodNotAvailable(format!("Failed to create remote desktop session: {e}"))
        })?;

        proxy
            .select_devices(&session, DeviceType::Keyboard.into(), None, PersistMode::DoNot)
            .await
            .map_err(|e| {
                InjectionError::PermissionDenied(format!("Failed to select keyboard device: {e}"))
            })?;

        proxy.start(&session, None).await.map_err(|e| {
            InjectionError::PermissionDenied(format!("Failed to start remote desktop session: {e}"))
        })?;

        // 3. Simulate Ctrl+V
        // KEY_LEFTCTRL = 29, KEY_V = 46
        proxy
            .notify_keyboard_keycode(&session, 29, KeyState::Pressed)
            .await
            .map_err(|e| InjectionError::MethodFailed(format!("Failed to press Left Ctrl: {e}")))?;
        proxy
            .notify_keyboard_keycode(&session, 46, KeyState::Pressed)
            .await
            .map_err(|e| InjectionError::MethodFailed(format!("Failed to press V: {e}")))?;
        proxy
            .notify_keyboard_keycode(&session, 46, KeyState::Released)
            .await
            .map_err(|e| InjectionError::MethodFailed(format!("Failed to release V: {e}")))?;
        proxy
            .notify_keyboard_keycode(&session, 29, KeyState::Released)
            .await
            .map_err(|e| InjectionError::MethodFailed(format!("Failed to release Left Ctrl: {e}")))?;

        Ok(())
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}
