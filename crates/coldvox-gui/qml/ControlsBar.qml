import QtQuick 6.5
import QtQuick.Controls 6.5

Item {
  id: bar
  signal stop()
  signal pauseResume()
  signal clear()
  signal openSettings()

  Rectangle {
    anchors.fill: parent
    color: Qt.rgba(0,0,0,0.25)
  }

  Row {
    spacing: 12
    anchors.verticalCenter: parent.verticalCenter
    anchors.left: parent.left; anchors.leftMargin: 16

    Button { text: "Stop"; opacity: 0.60; onHoveredChanged: opacity = hovered ? 1.0 : 0.60; onClicked: bar.stop() }
    Button { text: "Pause/Resume"; opacity: 0.60; onHoveredChanged: opacity = hovered ? 1.0 : 0.60; onClicked: bar.pauseResume() }
    Button { text: "Clear"; opacity: 0.60; onHoveredChanged: opacity = hovered ? 1.0 : 0.60; onClicked: bar.clear() }
  }

  Button {
    text: "Settings"
    anchors.verticalCenter: parent.verticalCenter
    anchors.right: parent.right; anchors.rightMargin: 16
    opacity: 0.60
    onHoveredChanged: opacity = hovered ? 1.0 : 0.60
    onClicked: bar.openSettings()
  }
}
