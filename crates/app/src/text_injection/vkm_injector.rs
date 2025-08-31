#![cfg(feature = "text-injection-vkm")]

use crate::text_injection::{InjectionError, TextInjector};
use crate::text_injection::types::InjectionMetrics;
use std::env;
use async_trait::async_trait;
use arboard::Clipboard;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_client::protocol::{wl_seat, wl_registry};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

#[derive(Debug)]
pub struct VkmInjector {
    metrics: InjectionMetrics,
}

impl VkmInjector {
    pub fn new() -> Self {
        Self::default()
    }

    fn compositor_may_support_vkm() -> bool {
        env::var_os("WAYLAND_DISPLAY").is_some()
    }
}

impl Default for VkmInjector {
    fn default() -> Self {
        Self {
            metrics: InjectionMetrics::default(),
        }
    }
}

struct AppData {
    seat: Option<wl_seat::WlSeat>,
    vk_manager: Option<ZwpVirtualKeyboardManagerV1>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_seat" => {
                    state.seat = Some(registry.bind::<wl_seat::WlSeat, _, _>(name, version, qh, ()));
                }
                "zwp_virtual_keyboard_manager_v1" => {
                    state.vk_manager = Some(registry.bind::<ZwpVirtualKeyboardManagerV1, _, _>(
                        name,
                        version,
                        qh,
                        (),
                    ));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardManagerV1, ()> for AppData {
     fn event(
        _state: &mut Self,
        _proxy: &ZwpVirtualKeyboardManagerV1,
        _event: <ZwpVirtualKeyboardManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardV1, ()> for AppData {
     fn event(
        _state: &mut Self,
        _proxy: &ZwpVirtualKeyboardV1,
        _event: <ZwpVirtualKeyboardV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}


#[async_trait]
impl TextInjector for VkmInjector {
    fn name(&self) -> &'static str {
        "Wayland-VKM"
    }

    fn is_available(&self) -> bool {
        Self::compositor_may_support_vkm()
    }

    async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        let mut clipboard = Clipboard::new().map_err(|e| {
            InjectionError::PermissionDenied(format!("Failed to initialize clipboard: {e}"))
        })?;
        clipboard.set_text(text.to_string()).map_err(|e| {
            InjectionError::PermissionDenied(format!("Failed to set clipboard text: {e}"))
        })?;

        tokio::task::spawn_blocking(move || -> Result<(), InjectionError> {
            let conn = Connection::connect_to_env().map_err(|e| InjectionError::MethodNotAvailable(e.to_string()))?;
            let mut event_queue = conn.new_event_queue();
            let qh = event_queue.handle();
            let display = conn.display();
            display.get_registry(&qh, ());

            let mut app_data = AppData { seat: None, vk_manager: None };
            event_queue.roundtrip(&mut app_data).unwrap();

            let seat = app_data.seat.ok_or_else(|| InjectionError::MethodNotAvailable("No wl_seat found".into()))?;
            let vk_manager = app_data.vk_manager.ok_or_else(|| InjectionError::MethodNotAvailable("No zwp_virtual_keyboard_manager_v1 found".into()))?;

            let virtual_keyboard = vk_manager.create_virtual_keyboard(&seat, &qh, ());

            // KEY_LEFTCTRL = 29, KEY_V = 46
            // Time is required by the protocol, but not used by the compositor.
            virtual_keyboard.key(0, 29, 1); // 1 for pressed
            virtual_keyboard.key(0, 46, 1);
            virtual_keyboard.key(0, 46, 0); // 0 for released
            virtual_keyboard.key(0, 29, 0);

            conn.flush().unwrap();

            Ok(())
        }).await.map_err(|e| InjectionError::Other(e.to_string()))?
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}
