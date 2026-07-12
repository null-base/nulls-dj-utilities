# null's DJ Utilities

rekordboxの現在再生中トラックを読み取り、VRChatのOSC chatboxとOBSのブラウザソースへ表示します。

## Development

Prerequisites:

- Rust
- Bun
- macOS or Windows

```bash
bun install
bun run dev
```

## Build

```bash
bun run build
```

Buildは実行OS向けの成果物を作ります。macOS版はmacOS上、Windows版はWindows上でビルドしてください。

## Source Layout

- `ui/`: Tauri web UI
- `src-tauri/src/monitor.rs`: rekordbox polling loop
- `src-tauri/src/rekordbox.rs`: rekordbox DB reader
- `src-tauri/src/osc.rs`: VRChat OSC sender
- `src-tauri/src/obs.rs`: OBS browser-source server
- `src-tauri/src/models.rs`: settings and shared data types

## OBS Browser Source

OBSのブラウザソースURL:

```text
http://0.0.0.0:17777/
```

OBSポートはアプリのSettingsで変更できます。ポート変更はアプリ再起動後に反映されます。

## Notes

- rekordbox DB path:
  - macOS: `~/Library/Application Support/Pioneer/rekordbox/master.db`
  - Windows: `%APPDATA%\Pioneer\rekordbox\master.db`
- Settings path:
  - macOS: `~/Library/Application Support/null's DJ Utilities/settings.json`
  - Windows: `%LOCALAPPDATA%\null's DJ Utilities\settings.json`
- OSC destination defaults to `127.0.0.1:9000`. Set OSC Host to another PC's IP address to send there.
- OSC Send Delay can delay new track messages by 0-30000 ms.
