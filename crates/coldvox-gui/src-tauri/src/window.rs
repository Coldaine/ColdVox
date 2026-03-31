use crate::contract::OverlaySnapshot;
use tauri::{LogicalSize, Size, WebviewWindow};

const COLLAPSED_WIDTH: f64 = 336.0;
const COLLAPSED_HEIGHT: f64 = 68.0;
const EXPANDED_WIDTH: f64 = 720.0;
const EXPANDED_HEIGHT: f64 = 448.0;

pub fn sync_window(window: &WebviewWindow, snapshot: &OverlaySnapshot) -> tauri::Result<()> {
    let (width, height) = if snapshot.expanded {
        (EXPANDED_WIDTH, EXPANDED_HEIGHT)
    } else {
        (COLLAPSED_WIDTH, COLLAPSED_HEIGHT)
    };

    window.set_size(Size::Logical(LogicalSize::new(width, height)))?;
    Ok(())
}
