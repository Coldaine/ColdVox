import { OverlayShell } from "./components/OverlayShell";
import { useOverlayShell } from "./hooks/useOverlayShell";

function App() {
  const {
    snapshot,
    clearTranscript,
    openSettings,
    setExpanded,
    startDemo,
    stopDemo,
    togglePause,
  } = useOverlayShell();

  return (
    <main className="app-frame">
      <OverlayShell
        snapshot={snapshot}
        onSetExpanded={setExpanded}
        onStartDemo={startDemo}
        onTogglePause={togglePause}
        onStop={stopDemo}
        onClear={clearTranscript}
        onSettings={openSettings}
      />
    </main>
  );
}

export default App;
