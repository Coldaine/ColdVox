import QtQuick 6.5
import QtQuick.Controls 6.5
import QtQuick.Layouts 6.5
import QtQuick.Window 6.5
import Qt.labs.platform 1.1
import Qt.labs.settings 1.1

// Top-level always-on-top overlay window
Window {
  id: root
  title: "ColdVox"
  visible: true
  color: "transparent"
  flags: Qt.Tool | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint

  // Bridge is provided by Rust main as a context property
  // Properties mirrored here for convenience
  property alias expanded: bridge.expanded
  property int stateCode: bridge.state
  // Audio level is not yet surfaced by the bridge; default to 0
  property int level: 0
  // Combine final + partial transcript for display; partial shown live, final accumulates
  property string displayTranscript: {
    var ft = bridge.final_transcript
    var pt = bridge.partial_transcript
    if (ft.length === 0) return pt
    if (pt.length === 0) return ft
    return ft + "\n" + pt
  }

  // Window geometry and persistence
  readonly property int collapsedWidth: 240
  readonly property int collapsedHeight: 48
  readonly property int minExpandedWidth: 600
  readonly property int maxExpandedWidth: Math.round(Screen.width * 0.6)
  readonly property int minExpandedHeight: 200
  readonly property int maxExpandedHeight: Math.round(Screen.height * 0.4)

  Settings {
    id: settings
    category: "coldvox"
    property real posX: (Screen.width - root.collapsedWidth) / 2
    property real posY: (Screen.height - root.collapsedHeight) / 2
    property real opacity: 0.30
  }

  x: settings.posX
  y: settings.posY
  width: expanded ? Math.min(Math.max(minExpandedWidth, Math.round(Screen.width * 0.5)), maxExpandedWidth)
                  : collapsedWidth
  height: expanded ? Math.min(Math.max(minExpandedHeight, Math.round(Screen.height * 0.25)), maxExpandedHeight)
                   : collapsedHeight

  onXChanged: settings.posX = x
  onYChanged: settings.posY = y

  // Dragging support (drag anywhere except over interactive controls in expanded panel)
  property point dragStart
  function startDrag(mouse) { dragStart = Qt.point(mouse.x, mouse.y) }
  function doDrag(mouse) {
    if (!mouse.buttons) return;
    root.x += mouse.x - dragStart.x
    root.y += mouse.y - dragStart.y
  }

  // Background chrome
  Rectangle {
    id: chrome
    anchors.fill: parent
    radius: expanded ? 16 : 24
    color: Qt.rgba(0.165, 0.165, 0.165, settings.opacity)
    border.width: 1
    border.color: Qt.rgba(1,1,1,0.10)
    layer.enabled: true
    layer.samples: 8
  }

  // Collapsed bar content
  CollapsedBar {
    id: collapsed
    anchors.fill: parent
    visible: !expanded
    stateCode: root.stateCode
    onOpenSettings: settingsWin.visible = true

    MouseArea {
      anchors.fill: parent
      onPressed: root.startDrag(mouse)
      onPositionChanged: root.doDrag(mouse)
      onClicked: bridge.expanded = !bridge.expanded
    }
  }

  // Expanded panel content
  ActivePanel {
    id: active
    anchors.fill: parent
    visible: expanded
    stateCode: root.stateCode
    level: root.level
    transcript: root.displayTranscript

    onStop: bridge.cmd_stop()
    onPauseResume: {
      if (bridge.state === 2) bridge.cmd_pause()       // Active -> Paused
      else if (bridge.state === 3) bridge.cmd_resume() // Paused -> Active
    }
    onClear: bridge.cmd_clear()
    onOpenSettings: settingsWin.visible = true

    // Drag from top activity area
    dragHandler: function(mouse) {
      if (mouse) {
        if (mouse.accepted) return;
        root.startDrag(mouse)
      }
    }
  }

  // Smooth resize animation
  Behavior on width  { NumberAnimation { duration: 300; easing.type: Easing.InOutQuad } }
  Behavior on height { NumberAnimation { duration: 300; easing.type: Easing.InOutQuad } }

  // Settings window dialog
  SettingsWindow {
    id: settingsWin
    opacityValue: settings.opacity
  }
  Connections {
    target: settingsWin
    function onOpacityValueChanged() { settings.opacity = settingsWin.opacityValue }
  }

  // Local shortcut (prototype). Global hotkey to be wired via backend later.
  Shortcut {
    sequences: [ "Ctrl+Shift+Space" ]
    onActivated: bridge.expanded = !bridge.expanded
  }

  // System tray icon and menu (works well on Plasma). On GNOME, requires extension.
  SystemTrayIcon {
    id: tray
    visible: Qt.platform.os === "linux"  // Basic platform gate for Linux
    icon.name: "microphone-sensitivity-high"
    tooltip: expanded ? "ColdVox: Visible" : "ColdVox"
    menu: Menu {
      MenuItem { text: expanded ? "Hide" : "Show"; onTriggered: bridge.expanded = !bridge.expanded }
      MenuSeparator {}
      MenuItem { text: (root.stateCode === 1) ? "Stop" : "Start"; onTriggered: { if (root.stateCode === 1) bridge.cmd_stop(); else bridge.cmd_start(); } }
      MenuItem { text: "Pause/Resume"; onTriggered: bridge.cmd_toggle_pause() }
      MenuItem { text: "Clear"; onTriggered: bridge.cmd_clear() }
      MenuSeparator {}
      MenuItem { text: "Settings"; onTriggered: settingsWin.visible = true }
      MenuSeparator {}
      MenuItem { text: "Quit"; onTriggered: Qt.quit() }
    }
  }


}
