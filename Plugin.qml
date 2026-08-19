import QtQuick
import Quickshell
import Quickshell.Io

Item {
  id: root

  property string omarchyPath: ""
  property var shell
  property var manifest
  property var pluginRegistry

  function open(payloadJson) {
    launcher.startDetached()
  }

  function close() {}

  Process {
    id: launcher
    command: [
      "sh", "-lc",
      "if command -v solitaire >/dev/null 2>&1; then exec solitaire; else notify-send --urgency=critical 'Solitaire is not installed' 'Install the native solitaire binary and try again.' 2>/dev/null || true; printf '%s\\n' 'Solitaire: native binary not found on PATH' >&2; exit 127; fi"
    ]
  }
}

