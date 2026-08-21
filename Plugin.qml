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
      "binary=$(command -v solitaire 2>/dev/null) || { notify-send --urgency=critical 'Solitaire is not installed' 'Install the native solitaire binary and try again.' 2>/dev/null || true; printf '%s\\n' 'Solitaire: native binary not found on PATH' >&2; exit 127; }; started=$(date +%s); \"$binary\"; status=$?; elapsed=$(($(date +%s)-started)); if [ $status -ne 0 ] && [ $elapsed -le 10 ]; then message=\"Solitaire failed during startup (exit $status). Run solitaire in a terminal for details.\"; notify-send --urgency=critical 'Solitaire could not start' \"$message\" 2>/dev/null || true; printf '%s\\n' \"$message\" >&2; fi; exit $status"
    ]
  }
}
