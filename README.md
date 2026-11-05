# Sheen

A Dynamic Island for Windows. It's a small capsule at the top of the screen that
changes shape for whatever is happening: media playback, volume and other
system OSDs, and later a file shelf, clipboard peek and scratchpad.

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
| `crates/sheen-win` | Windows services, one thread each: SMTC media sessions, Core Audio volume, full-screen detection, Win32 window styles. Stubs on other OSes. |
| `src-tauri` | Thin shell that wires the services to the core and the core to the webview, and owns hit-testing. |
| `ui` | Svelte 5 front-end. Renders a `Presentation` with spring physics and has no logic of its own. Opened in a plain browser, it runs a demo backend. |

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

## Development

Prerequisites (on Windows): Rust (stable, MSVC toolchain), Node 20+, pnpm,
the WebView2 runtime (preinstalled on Windows 11).

```sh
pnpm install
pnpm tauri dev        # run the app
pnpm dev              # UI only, in any browser, with a demo backend
                      #   add ?mode=idle|mediaCompact|osd|expanded to pin a state
cargo test -p sheen-core
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

**Phase 1: verify on hardware and polish**
- [ ] Verify on Windows 10 and 11: focus behaviour with WebView2, click-through, multi-DPI
- [ ] Measure the RAM baseline, tune WebView2 flags
- [ ] Re-position on display/DPI changes; choose a monitor
- [ ] Tray icon, autostart, settings file
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
