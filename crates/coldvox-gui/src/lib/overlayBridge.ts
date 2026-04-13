import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  OVERLAY_EVENT_NAME,
  type OverlayEvent,
  type OverlaySnapshot,
} from "../contracts/overlay";

export function getOverlaySnapshot(): Promise<OverlaySnapshot> {
  return invoke<OverlaySnapshot>("get_overlay_snapshot");
}

export function setOverlayExpanded(
  expanded: boolean,
): Promise<OverlaySnapshot> {
  return invoke<OverlaySnapshot>("set_overlay_expanded", { expanded });
}

export function startPipeline(): Promise<OverlaySnapshot> {
  return invoke<OverlaySnapshot>("start_pipeline");
}

export function togglePauseState(): Promise<OverlaySnapshot> {
  return invoke<OverlaySnapshot>("toggle_pause_state");
}

export function stopPipeline(): Promise<OverlaySnapshot> {
  return invoke<OverlaySnapshot>("stop_pipeline");
}

export function clearOverlayTranscript(): Promise<OverlaySnapshot> {
  return invoke<OverlaySnapshot>("clear_overlay_transcript");
}

export function openSettingsPlaceholder(): Promise<OverlaySnapshot> {
  return invoke<OverlaySnapshot>("open_settings_placeholder");
}

export function subscribeToOverlayEvents(
  onEvent: (event: OverlayEvent) => void,
): Promise<UnlistenFn> {
  return listen<OverlayEvent>(OVERLAY_EVENT_NAME, (event) => {
    onEvent(event.payload);
  });
}
