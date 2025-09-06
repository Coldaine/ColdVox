import QtQuick 6.5
import QtQuick.Controls 6.5
import QtQuick.Layouts 6.5

Item {
  id: panel
  // inputs
  property int stateCode: 0
  property int level: 0
  property string transcript: ""

  // outputs
  signal stop()
  signal pauseResume()
  signal clear()
  signal openSettings()

  // optional callback to forward drag from activity area
  property var dragHandler

  ColumnLayout {
    anchors.fill: parent
    spacing: 0

    // Activity area
    Item {
      id: top
      Layout.fillWidth: true
      Layout.preferredHeight: 40
      ActivityIndicator { anchors.fill: parent; stateCode: panel.stateCode; level: panel.level }
      MouseArea {
        anchors.fill: parent
        cursorShape: Qt.OpenHandCursor
        onPressed: if (panel.dragHandler) panel.dragHandler(mouse)
        onPositionChanged: if (panel.dragHandler) panel.dragHandler(mouse)
      }
    }

    // Transcript area
    Rectangle {
      Layout.fillWidth: true
      Layout.fillHeight: true
      color: "transparent"
      clip: true
      ScrollView {
        id: scroll
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true
        Text {
          id: transcriptText
          width: scroll.availableWidth
          wrapMode: Text.WordWrap
          color: "#F5F5F5"
          font.pixelSize: 16
          text: panel.transcript
          anchors.margins: 20
          anchors.fill: parent
          opacity: 1.0
          Behavior on opacity { NumberAnimation { duration: 200 } }
          onTextChanged: {
            opacity = 0.0
            fadeInTimer.start()
            scrollScrollToEnd.start()
          }
        }
        Timer { id: fadeInTimer; interval: 16; running: false; repeat: false; onTriggered: transcriptText.opacity = 1.0 }
        Timer { id: scrollScrollToEnd; interval: 16; running: false; repeat: false; onTriggered: scroll.contentItem.contentY = scroll.contentItem.contentHeight }
      }
    }

    // Controls
    Item {
      Layout.fillWidth: true
      Layout.preferredHeight: 40
      ControlsBar {
        anchors.fill: parent
        onStop: panel.stop()
        onPauseResume: panel.pauseResume()
        onClear: panel.clear()
        onOpenSettings: panel.openSettings()
      }
    }
  }
}
