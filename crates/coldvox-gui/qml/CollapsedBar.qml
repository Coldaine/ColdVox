import QtQuick 6.5
import QtQuick.Controls 6.5

Item {
  id: bar
  signal openSettings()
  // 0 idle, 1 recording, 2 processing, 3 complete
  property int stateCode: 0

  // Status LED center
  Rectangle {
    width: 8; height: 8; radius: 4
    anchors.verticalCenter: parent.verticalCenter
    anchors.horizontalCenter: parent.horizontalCenter
    color: stateCode === 1 ? "#00C853" : stateCode === 2 ? "#FFD54F" : stateCode === 3 ? "#00E5FF" : "#A0A0A0"
  }

  // Mic icon (left)
  Text {
    text: "🎤"
    font.pixelSize: 18
    color: Qt.rgba(1,1,1, 0.75)
    anchors.verticalCenter: parent.verticalCenter
    anchors.left: parent.left; anchors.leftMargin: 14
  }

  // Gear icon (right)
  Text {
    id: gear
    text: "\u2699"
    font.pixelSize: 18
    color: Qt.rgba(1,1,1, 0.75)
    anchors.verticalCenter: parent.verticalCenter
    anchors.right: parent.right; anchors.rightMargin: 14
    opacity: 0.75
    Behavior on opacity { NumberAnimation { duration: 120 } }
    MouseArea {
      anchors.fill: parent
      hoverEnabled: true
      onEntered: gear.opacity = 1.0
      onExited: gear.opacity = 0.75
      onClicked: bar.openSettings()
    }
  }
}
