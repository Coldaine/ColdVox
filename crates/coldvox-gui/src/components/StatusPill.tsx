import { type OverlayStatus } from "../contracts/overlay";

const LABELS: Record<OverlayStatus, string> = {
  idle: "Idle",
  listening: "Listening",
  processing: "Processing",
  ready: "Ready",
  error: "Error",
};

export function StatusPill({ status }: { status: OverlayStatus }) {
  return (
    <div className={`status-pill status-pill--${status}`}>
      <span className="status-pill__dot" aria-hidden="true" />
      <span>{LABELS[status]}</span>
    </div>
  );
}
