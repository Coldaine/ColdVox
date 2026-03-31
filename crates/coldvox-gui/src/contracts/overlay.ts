export const OVERLAY_EVENT_NAME = "coldvox://overlay";

export type OverlayStatus =
  | "idle"
  | "listening"
  | "processing"
  | "ready"
  | "error";

export interface OverlaySnapshot {
  expanded: boolean;
  status: OverlayStatus;
  paused: boolean;
  partialTranscript: string;
  finalTranscript: string;
  statusDetail: string;
  errorMessage: string | null;
}

export interface OverlayEvent {
  reason: string;
  snapshot: OverlaySnapshot;
}

export const DEFAULT_SNAPSHOT: OverlaySnapshot = {
  expanded: false,
  status: "idle",
  paused: false,
  partialTranscript: "",
  finalTranscript: "",
  statusDetail: "Overlay shell ready. Expand to inspect the seam.",
  errorMessage: null,
};
