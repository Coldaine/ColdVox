import QtQuick 6.5
import QtQuick.Controls 6.5
import QtQuick.Layouts 6.5

Window {
  id: win
  title: "ColdVox Settings"
  visible: false
  width: 480
  height: 600
  minimumWidth: 400
  minimumHeight: 500
  flags: Qt.Dialog | Qt.WindowStaysOnTopHint

  Rectangle { anchors.fill: parent; color: Qt.rgba(0.12, 0.12, 0.12, 0.95) }

  ScrollView {
    anchors.fill: parent
    contentWidth: availableWidth
    clip: true
    ColumnLayout {
      id: content
      width: parent.width
      spacing: 16
      padding: 20

      // Audio input device selection
      GroupBox {
        title: "Audio Input"
        Layout.fillWidth: true
        ColumnLayout {
          anchors.margins: 8
          anchors.fill: parent
          RowLayout {
            Layout.fillWidth: true
            Label { text: "Device"; Layout.preferredWidth: 120 }
            ComboBox { Layout.fillWidth: true; model: ["Default", "Device 1", "Device 2"] }
          }
        }
      }

      // Language selection
      GroupBox {
        title: "Language"
        Layout.fillWidth: true
        RowLayout {
          anchors.margins: 8
          anchors.fill: parent
          Label { text: "Language"; Layout.preferredWidth: 120 }
          ComboBox { Layout.fillWidth: true; model: ["Auto", "en-US", "en-GB", "de-DE", "fr-FR"] }
        }
      }

      // Hotkey configuration
      GroupBox {
        title: "Hotkey"
        Layout.fillWidth: true
        ColumnLayout {
          anchors.margins: 8
          anchors.fill: parent
          RowLayout {
            Layout.fillWidth: true
            Label { text: "Activation"; Layout.preferredWidth: 120 }
            TextField { Layout.fillWidth: true; placeholderText: "Ctrl+Shift+Space" }
          }
          Label { text: "Note: Global hotkeys require platform backend integration."; color: "#BBBBBB" }
        }
      }

      // Transparency level adjustment
      GroupBox {
        title: "Appearance"
        Layout.fillWidth: true
        ColumnLayout {
          anchors.margins: 8
          anchors.fill: parent
          RowLayout {
            Layout.fillWidth: true
            Label { text: "Transparency"; Layout.preferredWidth: 120 }
            Slider { Layout.fillWidth: true; from: 0.1; to: 0.8; value: 0.3 }
          }
          RowLayout {
            Layout.fillWidth: true
            Label { text: "Theme"; Layout.preferredWidth: 120 }
            ComboBox { Layout.fillWidth: true; model: ["Auto", "Light", "Dark"] }
          }
        }
      }

      // Auto-punctuation toggle
      GroupBox {
        title: "Transcription"
        Layout.fillWidth: true
        ColumnLayout {
          anchors.margins: 8
          anchors.fill: parent
          CheckBox { text: "Auto punctuation"; checked: true }
          RowLayout {
            Layout.fillWidth: true
            Label { text: "Output format"; Layout.preferredWidth: 120 }
            ComboBox { Layout.fillWidth: true; model: ["Plain Text", "Markdown", "Rich Text"] }
          }
        }
      }

      // API key configuration
      GroupBox {
        title: "API Configuration"
        Layout.fillWidth: true
        ColumnLayout {
          anchors.margins: 8
          anchors.fill: parent
          TextField { placeholderText: "API Key"; echoMode: TextInput.PasswordEchoOnEdit }
          TextField { placeholderText: "Endpoint URL" }
        }
      }

      // Buttons
      RowLayout {
        Layout.fillWidth: true
        spacing: 12
        Item { Layout.fillWidth: true }
        Button { text: "Close"; onClicked: win.visible = false }
      }
    }
  }
}
