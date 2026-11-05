# Sheen

A Dynamic Island for Windows, with a small built-in music player. It's a
capsule at the top of the screen that changes shape for whatever is happening:
media playback, volume and other system OSDs, and later a file shelf,
clipboard peek and scratchpad.

> **Status:** early scaffold. The architecture and the first vertical slice
> (media capsule, volume OSD, full-screen hiding, no-focus/click-through window)
> are in place. It has only been compile-checked for Windows so far, so expect
> rough edges on real hardware.

## Stack

**Rust + Tauri v2 + Svelte 5.** See [ADR 0001](docs/adr/0001-stack.md) for the
reasoning and the trade-offs that were accepted.

```
┌──────────────────────────── src-tauri (adapter) ────────────────────────────┐
│                                                                             │
│  sheen-win services ─┐                         ┌─► webview  (Svelte UI)     │
│  UI commands ────────┼─► Signal ─► reducer ────┤   sheen://presentation     │
│  hit-tester (hover) ─┘     channel  (Island)   └─► HitRegion ─► hit-tester  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Path | Role |
|---|---|
| `crates/sheen-core` | Pure Rust, no OS or UI deps. Event model, the `Island` state machine (priorities, OSD timeouts, expand/collapse), capsule geometry and hit region. Unit-tested on any OS. |
| `crates/sheen-win` | Windows services, one thread each: SMTC media sessions, Core Audio volume, full-screen detection, Win32 window styles, and publishing the built-in player to SMTC. Stubs on other OSes. |
| `crates/sheen-player` | Built-in player: library scan with tag cache (lofty), queue/shuffle/repeat, playback thread (rodio). Queue and library logic are unit-tested on any OS. |
| `src-tauri` | Thin shell that wires the services to the core and the core to the webview, and owns hit-testing. |
| `ui` | Svelte 5 front-end with two entry points: `index.html` (island) and `main.html` (library, player and settings). Opened in a plain browser, both run against a demo backend. |

### Window model

* **Fixed transparent canvas.** The native window is always 640×280 logical px
  at the top centre of the screen and is never resized. The capsule is
  animated inside it, so native resizes never have to be synced with the CSS
  animation and there's no flicker.
* **Dynamic click-through.** The window ignores the mouse
  (`WS_EX_TRANSPARENT`) everywhere except over the capsule. A small thread
  polls the cursor (60 Hz near the island, about 8 Hz otherwise) and toggles
  this. The region comes from `sheen-core`, the same source that gives the UI
  its target geometry. While a spring animation is running, the region is the
  union of the old and new shapes.
* **No focus stealing.** `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` plus Tauri's
  `focusable: false`. Clicking the island never deactivates the app you're
  typing in.
* **Game mode.** The island hides when the shell reports exclusive
  D3D/presentation mode, or when the foreground window covers the primary
  monitor.

### Built-in player

* **Formats:** MP3, FLAC, AAC/M4A, Ogg Vorbis and WAV. Only these decoders are compiled in.
* **Library:** scans the music folder (defaults to the Windows Music folder)
  in the background. Tags are cached in `library.json`, and a file is only
  re-read when its modification time changes, so rescans are close to
  instant.
* **Windows integration:** the player registers itself with SMTC, so media
  keys, the Windows media flyout and the island all control it exactly like
  Spotify. There is no separate code path.
* **Resource use:**
  * The audio device is opened only while playing and released after 30 s
    paused.
  * The main window is created when opened and destroyed when closed, so its
    webview uses no memory while hidden.
  * The track list is virtualised, so only the visible rows are in the DOM.
  * The playhead is interpolated in the UI instead of being streamed over IPC.

### Theme

Light or dark follows the Windows app theme automatically and switches live
when you change it in Windows. Settings → Appearance can force Light or Dark.
Both the main window and the island (black in dark mode, frosted white in
light mode) follow it.

Open the main window from the tray icon, or click the idle island pill.

## Development

Prerequisites (on Windows): Rust (stable, MSVC toolchain), Node 20+, pnpm,
the WebView2 runtime (preinstalled on Windows 11).

```sh
pnpm install
pnpm tauri dev        # run the app
pnpm dev              # UI only, in any browser, with a demo backend
                      #   /           island; ?mode=idle|mediaCompact|osd|expanded pins a state
                      #   /main.html  library/player window
cargo test -p sheen-core -p sheen-player
pnpm check            # svelte-check / TypeScript
pnpm tauri build      # NSIS + MSI installers
```

From Linux/macOS you can still type-check the whole Windows build:

```sh
rustup target add x86_64-pc-windows-msvc
cargo clippy --workspace --target x86_64-pc-windows-msvc
```

## Roadmap

**Phase 0: foundation (this scaffold)**
- [x] Core state machine + geometry, with tests
- [x] Transparent no-activate window, dynamic hit-testing, full-screen hiding
- [x] SMTC media: title/artist/art, play/pause/next/prev, seek
- [x] Volume OSD from Core Audio (any source), scroll-to-change-volume
- [x] Spring-animated capsule, compact / expanded / OSD views

**Phase 0.5: built-in player**
- [x] Library scan with incremental tag cache, search, virtualised list
- [x] Play/pause/next/prev/seek, shuffle, repeat (all/one), volume
- [x] Published to SMTC: media keys, Windows flyout and island
- [x] Main window (library, player bar, settings) created on demand; tray icon
- [x] Light/dark follows Windows, with an in-app override
- [ ] Gapless playback, playlists, album view

**Phase 1: verify on hardware and polish**
- [ ] Verify on Windows 10 and 11: focus behaviour with WebView2, click-through, multi-DPI
- [ ] Measure the RAM baseline, tune WebView2 flags
- [ ] Re-position on display/DPI changes; choose a monitor
- [ ] Autostart
- [ ] Real level meter for the mini waveform (`IAudioMeterInformation`)

**Phase 2: System OSD**
- [ ] Brightness (WMI `WmiMonitorBrightnessEvent`)
- [ ] Mic mute (capture endpoint `IAudioEndpointVolume`), Caps/Num Lock
- [ ] Bluetooth device battery on connect
- [ ] Suppress the native volume flyout (no public API, needs research)
- [ ] Output device switcher (speakers ↔ headphones)

**Phase 3: Productivity shelf**
- [ ] Drop shelf (global drag detection → open shelf → OLE drag back out)
- [ ] Clipboard peek (`AddClipboardFormatListener`)
- [ ] Micro scratchpad. This one needs a deliberate focus hand-off, because the
      window is otherwise non-activating.

## License

MIT
