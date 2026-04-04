import QtQuick 6.5

Item {
  id: root
  // 0 idle, 1 recording, 2 processing, 3 complete
  property int stateCode: 0
  // 0..100 level
  property int level: 0
  // internal phase for animation
  property real phase: 0

  // Background gradient hinting state
  Rectangle {
    anchors.fill: parent
    color: "transparent"
    border.color: "transparent"
    gradient: Gradient {
      GradientStop { position: 0.0; color: stateCode === 1 ? "#44FF5252" : stateCode === 2 ? "#44FFB300" : "#443CCB5A" }
      GradientStop { position: 1.0; color: "#00000000" }
    }
  }

  // Waveform bars
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
        color: stateCode === 2 ? "#FFD24D" : stateCode === 1 ? "#FF6E6E" : "#00D084"
        // mix level with a sin sweep for lively look
        height: 6 + (level / 4) * 0.5 + Math.abs(Math.sin((index + root.phase) / 2)) * (10 + level / 6)
        Behavior on height { NumberAnimation { duration: 80 } }
      }
    }
  }

  Timer {
    interval: 30; running: true; repeat: true
    onTriggered: root.phase = root.phase + 0.7
  }
}
