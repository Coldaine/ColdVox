import { useCallback, useEffect, useState } from "react";
import {
  DEFAULT_SNAPSHOT,
  type OverlaySnapshot,
} from "../contracts/overlay";
import {
  clearOverlayTranscript,
  getOverlaySnapshot,
  openSettingsPlaceholder,
  setOverlayExpanded,
  startPipeline,
  stopPipeline,
  subscribeToOverlayEvents,
  togglePauseState,
} from "../lib/overlayBridge";

function messageFromError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Unknown bridge failure.";
}

export function useOverlayShell() {
  const [snapshot, setSnapshot] = useState<OverlaySnapshot>(DEFAULT_SNAPSHOT);

  useEffect(() => {
    let active = true;

    void getOverlaySnapshot()
      .then((initialSnapshot) => {
        if (active) {
          setSnapshot(initialSnapshot);
        }
      })
      .catch((error: unknown) => {
        if (active) {
          setSnapshot((current) => ({
            ...current,
            expanded: true,
            status: "error",
            statusDetail: "Unable to reach the Tauri host shell.",
            errorMessage: messageFromError(error),
          }));
        }
      });

    const unlistenPromise = subscribeToOverlayEvents((event) => {
      if (active) {
        setSnapshot(event.snapshot);
      }
    });

    return () => {
      active = false;
      void unlistenPromise.then((unlisten) => {
        unlisten();
      });
    };
  }, []);

  const runCommand = useCallback(
    async (command: () => Promise<OverlaySnapshot>) => {
      try {
        const nextSnapshot = await command();
        setSnapshot(nextSnapshot);
      } catch (error: unknown) {
        setSnapshot((current) => ({
          ...current,
          expanded: true,
          status: "error",
          statusDetail: "The host shell rejected the latest command.",
          errorMessage: messageFromError(error),
        }));
      }
    },
    [],
  );

  return {
    snapshot,
    setExpanded: (expanded: boolean) => runCommand(() => setOverlayExpanded(expanded)),
    startPipeline: () => runCommand(startPipeline),
    togglePause: () => runCommand(togglePauseState),
    stopPipeline: () => runCommand(stopPipeline),
    clearTranscript: () => runCommand(clearOverlayTranscript),
    openSettings: () => runCommand(openSettingsPlaceholder),
  };
}
