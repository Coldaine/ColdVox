import { fireEvent, render, screen } from "@testing-library/react";
import { OverlayShell } from "./OverlayShell";
import { DEFAULT_SNAPSHOT, type OverlaySnapshot } from "../contracts/overlay";

function createSnapshot(overrides: Partial<OverlaySnapshot> = {}): OverlaySnapshot {
  return {
    ...DEFAULT_SNAPSHOT,
    expanded: true,
    ...overrides,
  };
}

describe("OverlayShell", () => {
  it("renders final and partial transcript lanes distinctly", () => {
    render(
      <OverlayShell
        snapshot={createSnapshot({
          status: "listening",
          partialTranscript: "partial words stay provisional",
          finalTranscript: "final words stay dominant",
        })}
        onSetExpanded={vi.fn().mockResolvedValue(undefined)}
        onStartDemo={vi.fn().mockResolvedValue(undefined)}
        onTogglePause={vi.fn().mockResolvedValue(undefined)}
        onStop={vi.fn().mockResolvedValue(undefined)}
        onClear={vi.fn().mockResolvedValue(undefined)}
        onSettings={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(screen.getByTestId("final-transcript")).toHaveTextContent(
      "final words stay dominant",
    );
    expect(screen.getByTestId("partial-transcript")).toHaveTextContent(
      "partial words stay provisional",
    );
    expect(screen.getByText("Listening")).toBeInTheDocument();
  });

  it("wires the required controls", () => {
    const onStop = vi.fn().mockResolvedValue(undefined);
    const onTogglePause = vi.fn().mockResolvedValue(undefined);
    const onClear = vi.fn().mockResolvedValue(undefined);
    const onSettings = vi.fn().mockResolvedValue(undefined);

    render(
      <OverlayShell
        snapshot={createSnapshot({ status: "processing" })}
        onSetExpanded={vi.fn().mockResolvedValue(undefined)}
        onStartDemo={vi.fn().mockResolvedValue(undefined)}
        onTogglePause={onTogglePause}
        onStop={onStop}
        onClear={onClear}
        onSettings={onSettings}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(onStop).toHaveBeenCalledTimes(1);
    expect(onTogglePause).toHaveBeenCalledTimes(1);
    expect(onClear).toHaveBeenCalledTimes(1);
    expect(onSettings).toHaveBeenCalledTimes(1);
  });
});
