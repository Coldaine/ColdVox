import { useCallback, useEffect, useState, useRef } from "react";
import {
  DEFAULT_SNAPSHOT,
  type OverlaySnapshot,
} from "../contracts/overlay";
import {
  clearOverlayTranscript,
  getOverlaySnapshot,
  openSettingsPlaceholder,
  setOverlayExpanded,
  startDemoDriver,
  stopDemoDriver,
  subscribeToOverlayEvents,
  togglePauseState,
  updatePartialTranscript,
  updateFinalTranscript,
  setOverlayProcessing,
  setOverlayListening,
  stopOverlayCapture,
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

  // Debounce-flush partial transcript updates to avoid flooding the shell on rapid STT output.
  const pendingPartialRef = useRef<string | null>(null);
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flushPartial = useCallback(() => {
    const text = pendingPartialRef.current;
    if (text === null) return;
    pendingPartialRef.current = null;
    if (flushTimerRef.current !== null) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    void runCommand(() => updatePartialTranscript(text));
  }, [runCommand]);

  const queuePartialTranscript = useCallback(
    (text: string) => {
      pendingPartialRef.current = text;
      if (flushTimerRef.current !== null) {
        clearTimeout(flushTimerRef.current);
      }
      // Flush after 80 ms of no new partials — balances latency vs. reduce repaints.
      flushTimerRef.current = setTimeout(flushPartial, 80);
    },
    [flushPartial],
  );

  const cancelPendingPartial = useCallback(() => {
    pendingPartialRef.current = null;
    if (flushTimerRef.current !== null) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }
  }, []);

  // Cancel any pending partial flush when the component unmounts.
  useEffect(() => {
    return () => {
      cancelPendingPartial();
    };
  }, [cancelPendingPartial]);

  return {
    snapshot,
    setExpanded: (expanded: boolean) => runCommand(() => setOverlayExpanded(expanded)),
    startDemo: () => runCommand(startDemoDriver),
    togglePause: () => runCommand(togglePauseState),
    stopDemo: () => runCommand(stopDemoDriver),
    clearTranscript: () => runCommand(clearOverlayTranscript),
    openSettings: () => runCommand(openSettingsPlaceholder),
    // Pipeline wiring — for real STT integration.
    // queuePartialTranscript debounces rapid partials; flushPartial sends immediately.
    queuePartialTranscript,
    updateFinalTranscript: (text: string) => {
      cancelPendingPartial();
      return runCommand(() => updateFinalTranscript(text));
    },
    setOverlayProcessing: () => {
      cancelPendingPartial();
      return runCommand(setOverlayProcessing);
    },
    setOverlayListening: () => {
      cancelPendingPartial();
      return runCommand(setOverlayListening);
    },
    stopOverlayCapture: () => {
      cancelPendingPartial();
      return runCommand(stopOverlayCapture);
    },
  };
}
