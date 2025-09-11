import QtQuick 6.5
import QtQuick.Controls 6.5
import QtQuick.Layouts 6.5
import Qt5Compat.GraphicalEffects
import Qt.labs.settings 1.1

// Main always-on-top overlay window with collapsed and expanded states.
Window {
  id: root
  title: "ColdVox"
  visible: true
  flags: Qt.Tool | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
  color: "transparent"
  // DPI aware sizing via scaleFactor; can be refined per-platform
  property real scaleFactor: Screen.pixelDensity > 0 ? Screen.pixelDensity / 4.0 : 1.0

  // Collapsed spec: 240x48, 30% opacity, rounded 24px
  // Expanded spec: responsive 600-800 width, 200-400 height
  property bool expanded: settings.expanded

  // Basic state vars (stubbed if no Rust bridge provided)
  // state: 0=ready, 1=recording, 2=processing
  property int st: 0
  property int level: 0
  property string transcript: ""

  // Persist window position, size, and state
  Settings {
    id: settings
    category: "coldvox"
    property real x: (Screen.width - 240) / 2
    property real y: (Screen.height - 48) / 2
    property bool expanded: false
    property int expandedWidth: 600
    property int expandedHeight: 200
  }

  x: settings.x
  y: settings.y
  width: expanded ? settings.expandedWidth : 240
  height: expanded ? settings.expandedHeight : 48

  // Save changes back to settings
  onXChanged: settings.x = x
  onYChanged: settings.y = y
  onExpandedChanged: settings.expanded = expanded
  // Only save dimensions when in the expanded state
  onWidthChanged: if (expanded) settings.expandedWidth = width
  onHeightChanged: if (expanded) settings.expandedHeight = height

  // Drag anywhere in top activity area or collapsed bar
  property point dragStart
  function startDrag(mouse) { dragStart = Qt.point(mouse.x, mouse.y) }
  function doDrag(mouse) {
    if (!mouse.buttons) return;
    root.x += mouse.x - dragStart.x
    root.y += mouse.y - dragStart.y
  }

  // Top-level background with acrylic-ish effect (fallback to semi-transparent)
  Rectangle {
    id: bg
    anchors.fill: parent
    radius: expanded ? 16 : 24
    color: expanded ? Qt.rgba(0.16, 0.16, 0.16, 0.30) : Qt.rgba(0.16, 0.16, 0.16, 0.30)
    border.width: expanded ? 1 : 0
    border.color: Qt.rgba(1, 1, 1, 0.10)
    layer.enabled: true
    layer.smooth: true
    layer.samples: 4
    // Subtle drop shadow
    Rectangle {
      anchors.fill: parent
      radius: parent.radius
      color: "transparent"
      layer.enabled: true
      layer.effect: DropShadow {
        horizontalOffset: 0
        verticalOffset: 6
        radius: 16
        samples: 17
        color: Qt.rgba(0, 0, 0, 0.35)
        source: bg
      }
    }
  }

  // Collapsed idle bar
  Item {
    id: collapsedBar
    anchors.fill: parent
    visible: !expanded

    // Status LED
    Rectangle {
      id: statusLed
      width: 8; height: 8
      radius: 4
      color: st === 1 ? "#FF4D4D" : (st === 2 ? "#FFD24D" : "#00D084")
      anchors.verticalCenter: parent.verticalCenter
      anchors.horizontalCenter: parent.horizontalCenter
    }

    // Mic icon (left)
    Text {
      id: micIcon
      text: "🎤"
      color: Qt.rgba(1,1,1, 0.70)
      font.pixelSize: 18
      anchors.verticalCenter: parent.verticalCenter
      anchors.left: parent.left
      anchors.leftMargin: 14
      Behavior on color { ColorAnimation { duration: 150 } }
    }

    // Gear icon (right)
    Text {
      id: gearIcon
      text: "⚙"
      color: Qt.rgba(1,1,1, 0.70)
      font.pixelSize: 18
      anchors.verticalCenter: parent.verticalCenter
      anchors.right: parent.right
      anchors.rightMargin: 14
      opacity: 0.70
      Behavior on opacity { NumberAnimation { duration: 100 } }
      MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        onEntered: gearIcon.opacity = 1.0
        onExited: gearIcon.opacity = 0.70
        onClicked: settingsWindow.visible = true
      }
    }

    // Click anywhere to expand/start
    MouseArea {
      anchors.fill: parent
      onPressed: root.startDrag(mouse)
      onPositionChanged: root.doDrag(mouse)
      onClicked: {
        expanded = true
        st = 1 // start recording state visually
        if (typeof bridge !== 'undefined' && bridge.cmd_start)
          bridge.cmd_start()
      }
    }
  }

  // Expanded active transcription panel
  ColumnLayout {
    id: expandedPanel
    anchors.fill: parent
    anchors.margins: 0
    spacing: 0
    visible: expanded

    // Activity indicator area (40px)
    Rectangle {
      id: activity
      Layout.fillWidth: true
      Layout.preferredHeight: 40
      color: "transparent"

      // Waveform style bars
      Row {
        id: bars
        anchors.fill: parent
        anchors.margins: 12
        spacing: 6
        Repeater {
          model: 24
          delegate: Rectangle {
            width: (bars.width - (bars.spacing * (model - 1))) / model
            radius: 2
            anchors.bottom: parent.bottom
            color: st === 2 ? "#FFD24D" : (st === 1 ? "#FF4D4D" : "#00D084")
            height: 8 + Math.abs(Math.sin((index + perfTimer.msec/50) / 2)) * 22
            Behavior on height { NumberAnimation { duration: 100 } }
          }
        }
      }

      // Dragging from top bar
      MouseArea {
        anchors.fill: parent
        onPressed: root.startDrag(mouse)
        onPositionChanged: root.doDrag(mouse)
      }
    }

    // Transcript area
    Rectangle {
      Layout.fillWidth: true
      Layout.fillHeight: true
      color: "transparent"
      ScrollView {
        id: scroll
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true
        ScrollBar.vertical.policy: ScrollBar.AsNeeded
        Column {
          id: transcriptColumn
          width: scroll.availableWidth
          spacing: 6
          padding: 20
          Text {
            id: transcriptText
            width: parent.width
            wrapMode: Text.WordWrap
            color: "#F5F5F5"
            font.pixelSize: 16
            lineHeight: 1.5
            text: root.transcript
            Behavior on opacity { NumberAnimation { duration: 200 } }
            onTextChanged: scroll.scrollToBottom()
          }
        }
        function scrollToBottom() {
          contentItem.contentY = contentItem.contentHeight
        }
      }
    }

    // Control bar (40px)
    Rectangle {
      Layout.fillWidth: true
      Layout.preferredHeight: 40
      color: Qt.rgba(0,0,0,0.25)
      RowLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 10

        // Stop
        ControlButton { label: "⏹"; onClicked: { st = 2; if (typeof bridge !== 'undefined' && bridge.cmd_stop) bridge.cmd_stop() } }
        // Pause / Resume
        ControlButton {
          label: st === 1 ? "⏸" : "▶"
          onClicked: {
            if (typeof bridge !== 'undefined' && bridge.cmd_toggle_pause) bridge.cmd_toggle_pause()
            st = (st === 1) ? 0 : 1
          }
        }
        // Clear
        ControlButton { label: "🗑"; onClicked: { transcript = ""; if (typeof bridge !== 'undefined' && bridge.cmd_clear) bridge.cmd_clear() } }

        Item { Layout.fillWidth: true }

        // Settings (right-aligned)
        ControlButton { label: "⚙"; onClicked: settingsWindow.visible = true }
      }
    }
  }

  // Transitions for expand / collapse
  states: [
    State { name: "collapsed"; when: !expanded },
    State { name: "expanded"; when: expanded }
  ]

  transitions: [
    Transition {
      from: "collapsed"; to: "expanded"
      NumberAnimation { properties: "width,height"; duration: 300; easing.type: Easing.InOutQuad }
      ColorAnimation { target: bg; property: "color"; duration: 300 }
    },
    Transition {
      from: "expanded"; to: "collapsed"
      NumberAnimation { properties: "width,height"; duration: 300; easing.type: Easing.InOutQuad }
      ColorAnimation { target: bg; property: "color"; duration: 300 }
    }
  ]

  // Timer to animate waveform bars
  Timer { id: perfTimer; interval: 33; running: expanded; repeat: true; onTriggered: {} }

  // Settings window
  SettingsWindow { id: settingsWindow }

  // Keyboard shortcut (local to window) for expand/collapse
  Shortcut {
    sequences: [ StandardKey.Cancel, "Ctrl+Shift+Space" ]
    onActivated: expanded = !expanded
  }
}

// Simple flat button
component ControlButton: Rectangle {
  id: btn
  implicitWidth: 40
  implicitHeight: 28
  radius: 6
  color: Qt.rgba(1,1,1,0.10)
  property alias label: lbl.text
  signal clicked()
  opacity: 0.60
  Behavior on opacity { NumberAnimation { duration: 100 } }
  Text {
    id: lbl
    anchors.centerIn: parent
    color: "#F5F5F5"
    font.pixelSize: 14
  }
  MouseArea {
    anchors.fill: parent
    hoverEnabled: true
    onEntered: btn.opacity = 1.0
    onExited: btn.opacity = 0.60
    onClicked: btn.clicked()
  }
}
