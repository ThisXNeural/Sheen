# Sheen

A Dynamic Island for Windows, with a built-in music player. It's a capsule
at the top of the screen that changes shape for whatever is happening:
media playback, volume, background downloads and file copies, a file shelf
you can drag things onto, clipboard peeks, and a quick note on a shortcut.

> **Status:** early. The Rust code is unit-tested where it is platform
> independent and compile- and lint-checked for Windows. The UI is
> layout-tested end to end in Chromium. The Windows integrations (SMTC, UI
> Automation, hotkeys, shell drag-and-drop) have not been run on real Windows
> yet, so expect rough edges there.

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
| `crates/sheen-core` | Pure Rust, no OS or UI deps. Event model, the `Island` state machine (priorities, timeouts, panels), capsule geometry and hit region, the transfer ETA estimator, shortcut parsing. Unit-tested on any OS. |
| `crates/sheen-win` | Windows services, one thread each: SMTC media sessions, Core Audio volume, full-screen detection, transfers (downloads/copies), clipboard listener, global hotkeys, shell drag-and-drop, Win32 window styles, and publishing the built-in player to SMTC. Stubs on other OSes. |
| `crates/sheen-player` | Built-in player: library scan with tag cache (lofty), cover thumbnails, playlists and liked songs, queue/shuffle/repeat, playback thread (rodio). Logic is unit-tested on any OS. |
| `src-tauri` | Thin shell that wires the services to the core and the core to the webview, and owns hit-testing. |
| `ui` | Svelte 5 front-end with three entry points: `index.html` (island), `main.html` (library, player, settings) and `note.html` (quick note). In a plain browser they run against demo backends. |
| `tests/e2e` | Playwright layout tests over every view, island mode, theme and window size. |

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

### Downloads and file transfers

Windows has no API for reading another app's progress, so each source is read
the way it exposes itself. Browser downloads always show. File Explorer and
other progress dialogs show only while the dialog is minimized or behind other
windows, because otherwise the dialog itself is already in front of you.
Leftover `Unconfirmed *.crdownload` files from cancelled downloads are
ignored, because a download only counts while its file keeps growing.

| Source | How it's read | Shows |
|---|---|---|
| Chrome, Edge, Brave, Opera, Vivaldi | the `.crdownload` file's size, plus the expected size from the browser's `History` database (read in place with Windows' own `winsqlite3.dll`) | progress, speed, ETA |
| Firefox | the `.part` file's size | size and speed (Firefox doesn't record the total until the download is done) |
| File Explorer copy/move (incl. USB, network, phones) | the copy dialog's progress bar, through UI Automation | progress, ETA |
| IDM, 7-Zip, WinRAR, installers | standard Win32 progress-bar dialogs (`PBM_GETPOS`) | progress, ETA |

Browser downloads are found in the Downloads folder (watched with
`ReadDirectoryChangesW`, so it costs nothing while idle) and, for Chromium
browsers, anywhere else through the browser's own record of active downloads.
That record is re-read only when its file changes on disk. Window-based
sources are polled every 1.5 s, which is one `EnumWindows` pass of well under
a millisecond.

**ETA.** A naive `remaining / speed` jumps around, so the estimate (in
`sheen-core/src/eta.rs`) works in three layers:

1. A Kalman filter tracks throughput. Its noise is relative to the rate, so it
   adapts the same way at 50 KB/s and at 500 MB/s.
2. The filtered rate is blended with the average since the start.
3. The shown time counts down smoothly while new estimates agree with it
   (within 15%). When they disagree, it eases 30% of the way toward the new
   estimate each second, so it never jumps.

A transfer with no progress for 4 s shows as stalled instead of counting up.
Unit tests cover steady speed, ±40% noise, stalls and speed changes.

### Using the island

* **Playing:** the island shrinks to just the artwork and level bars, with the
  song's progress as a hairline along the bottom. Hover it for the title and
  artist, and click it for the full card. Clicking anywhere else closes the
  card at once.
* **Hide:** hovering shows a small × beside the island, which hides it. Hover
  the same spot at the top of the screen to bring it back.
* **Size:** Settings → Island → Size (Small to Extra large).

### Shelf and clipboard

* **Shelf.** While you drag files anywhere (detected through the shell's drag
  image), the island opens as a drop target. Files dropped on it stay parked:
  drag them back out to any app (the shell's own `SHDoDragDrop`, so they behave
  exactly like a drag from Explorer), double-click to open, or remove them.
  While files are parked, a small round shelf button with a count sits next
  to the island; click it to open the shelf.
* **Clipboard peek.** A small preview (text, image thumbnail or file names)
  appears when you copy something. It is event-driven
  (`AddClipboardFormatListener`). Content that password managers mark as
  private is never read.

### Quick note

Press **Scroll Lock** (changeable in Settings → Quick note, recorded by
pressing the keys) and a small note card drops down below the island. Text
autosaves to `note.txt` in Sheen's data folder. Esc or the shortcut closes it.
It also closes when you click elsewhere, unless pinned. The window is created
on first use and then only hidden, so later opens are instant.

### Built-in player

* **Library views:** Songs (sortable, searchable), Albums (grid and album
  page), Artists, Liked Songs and Playlists, each with a rounded cover
  thumbnail next to the title.
* **Playback:** play/pause, next/previous, seek, shuffle, repeat (all/one),
  volume and mute, and an "Up next" queue with Play next, Add to queue, jump
  and remove. The session (queue, track, position, modes) is restored on
  start.
* **Right-click menu:** works on one or several selected songs (Ctrl/Shift
  click): play, play next, add to queue, add to a playlist (or a new one),
  like, go to album or artist, show in Explorer.
* **Keyboard:**
  * Space: play/pause
  * Ctrl+←/→: previous/next
  * Ctrl+↑/↓: volume
  * Ctrl+F: search
  * Ctrl+L: like
  * Enter: play the selected song
* **Cover art:** the embedded picture, or `cover.jpg`/`folder.png` etc. next
  to the files. Covers are downscaled once to small JPEGs, cached on disk, and
  served to `<img>` tags through a `sheen-art://` protocol, so they load
  lazily and are never sent as base64 over IPC.
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
pnpm test:e2e         # layout tests: builds the UI and drives it in Chromium
pnpm check            # svelte-check / TypeScript
pnpm tauri build      # NSIS + MSI installers
```

From Linux/macOS you can still type-check the whole Windows build:

```sh
rustup target add x86_64-pc-windows-msvc
cargo clippy --workspace --target x86_64-pc-windows-msvc
```

### UI tests

`pnpm test:e2e` opens every main-window page, island mode and the note in
light and dark themes, at the minimum, default and a large window size, with
long and right-to-left (Persian) names. On every rendered element it checks:

* **escape:** nothing is cut off by a clipping ancestor or the window edge.
* **spill:** no element sticks out of its parent.
* **text-spill:** no text runs past its own box.
* **clipped-text:** text is never clipped without an ellipsis or line clamp.
* **overlap:** flex and grid children never overlap.
* **squashed:** nothing with text is squeezed to zero size.
* **icon:** every icon renders inside its 24×24 box.

A self-test makes sure the checker catches each of these problems.

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
- [x] Albums, artists, playlists, liked songs, queue panel, context menus
- [x] Cover thumbnails (embedded or folder art), session restore
- [ ] Gapless playback, playlist drag-to-reorder

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

**Phase 3: Productivity**
- [x] Drop shelf (global drag detection → drop target → shell drag back out)
- [x] Clipboard peek (`AddClipboardFormatListener`)
- [x] Quick note on a global shortcut
- [x] Downloads and file transfers with a smoothed ETA
- [ ] Downloads saved outside the Downloads folder; a browser extension for
      exact totals in Firefox

## License

MIT
