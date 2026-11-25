# AGENTS.md

Guide for coding agents and contributors working in `/home/alekra1/greeting`.

## Project Overview

`greeting` is a single-binary Rust CLI (installed to `~/.cargo/bin/greeting`) invoked on interactive shell startup from `~/.bashrc`. It renders a compact `15`-row animated dark-fantasy ASCII greeting with a native text system HUD on the left and a high-density optical-bloom graphic on the right:

- **Kitty Graphics Protocol** (`f=32`, zlib-compressed RGBA) on `ghostty`, `kitty`, `wezterm`
- **Native transparent Sixel** (`P2=1`, 1:1 native pixels, adaptive 240-color palette) on `foot`, `mlterm`, `contour`
- **Pure 24-bit TrueColor ANSI text** fallback in `tmux`, `screen`, SSH/non-graphics terminals, narrow panes (`< 46` cols), or `--text`

It also retains the legacy collectible pet subcommands (`pet`, `roll`, `collection`, `set`, `unset`, `catalog`).

---

## Build, Install & Verify

```bash
# Build & install locally (links system libz via #[link(name = "z")])
cd /home/alekra1/greeting && cargo install --path . --offline
```

**Requirements before finishing any change:**
1. **Zero compiler warnings** (`cargo install --path . --offline`).
2. **Never launch visible GUI terminal windows (`foot`, `ghostty`) to test** — always verify **headlessly**:
   - **Visual inspection of HD RGBA bloom:** run `/home/alekra1/.cargo/bin/greeting --scene <name> --dump-rgba /tmp/hd_<name>.rgba` (`462 × 330 × 4` straight RGBA bytes), composite over `#0b0d11` into a PNG via Python `cairo`, inspect with your image viewer, and delete `/tmp/hd_*.rgba` afterwards.
   - **Visual inspection of Sixel output:** run `greeting --scene <name> --static` inside a Python `pty.openpty()` with `TERM=foot` and `TIOCSWINSZ` `(52, 110, 1210, 1352)`, decode the `\x1bP0;1;0q...\x1b\\` Sixel stream to PNG, and inspect it.
   - **Lifecycle & resize verification:** test `bash -i` in a headless PTY for both `foot` (Sixel) and `ghostty` (Kitty Graphics), asserting:
     - Live frames emit immediately on boot (`C=1, q=2` for Kitty; `P2=1` `462×330` for Sixel)
     - Animation continues while typing without pressing Enter
     - Window resize cleanly deletes/redraws without ghost lines
     - Pressing Enter executes the command and stops the background daemon (`0` frames afterwards)
     - `--text` emits pure TrueColor ANSI with no `\x1b_G` or `\x1bP` sequences

---

## Codebase Map

| File | Role |
|---|---|
| `src/main.rs` | `clap` CLI definition (`--scene`, `--interval`, `--static`, `--fg`, `--text`, hidden `--daemon`, `--shell-pid`, `--dump-rgba`, `--dump-time`) and dispatch |
| `src/scenes.rs` | `Scene` enum (6 scenes), `70×17` pure-text fallback renderer, hybrid HUD (`HUD_WIDTH = 23`, graphic at col `24`), `compute_hd_layout`, `start_background_banner`, `run_daemon`, `run_foreground`, `print_static`, `dump_hd_rgba` |
| `src/scenes_hd.rs` | All 6 `66×30` HD scenes (`render_hd_eclipse`, `render_hd_bonfire`, `render_hd_torii`, `render_hd_portal`, `render_hd_grimoire`, `render_hd_nether`) and `scene_hud_style` |
| `src/gfx.rs` | `HD_COLS = 66`, `HD_ROWS = 30`, `IMG_W = 462`, `IMG_H = 330`, `HdGrid`, `BloomRenderer` (2-scale separable half-res box blur + HDR soft-knee bloom), Kitty Graphics zlib+base64 emitter, native Sixel adaptive-palette encoder, `detect_graphics_proto` (env + `/proc/<pid>/comm` ancestor walk) |
| `src/font_atlas.rs` | Static `7×11` antialiased `JetBrainsMono` glyph alpha atlas (`111` glyphs: ASCII `32..=126` + `· ° § ◈ ◇ ─ │ ═ ║ ╬ █ ▓ ▒ ░ ▄ ▀`) |
| `src/sysinfo.rs` | Gathers OS, kernel, local IP, and package count |
| `src/display.rs`, `src/pets.rs`, `src/collection.rs`, `src/state.rs` | Classic pet drop system (keep working; do not break) |

---

## Design Invariants & Hard Rules

1. **Approved 6-scene lineup:** `eclipse` (Eclipse Knight), `bonfire` (Ruined Bonfire), `torii` (Moonlit Torii / Shrine Gate), `portal` (Astral Gate), `grimoire` (Arcane Grimoire), `nether` (Nether Portal). Default rotation interval is `5` seconds (random other scene).
2. **Clean HUD only:** Left HUD displays ONLY the scene title, horizontal rule, and `user`, `os`, `kern`, `ip`, `pkgs`. **Never** add meaningless tagline/filler lines or interactive control footers.
3. **Compact spacing:** Keep `HUD_WIDTH = 23`, graphic starting at col `24` (`HUD_WIDTH + 1`), `HD_BANNER_ROWS = 15`, and `HD_COLS = 66` (`462×330` px) with scene artwork anchored close to the left edge of the HD canvas (no dead padding between HUD and art).
4. **No ANSI background fake-bloom:** Never use `\x1b[48;2;...m` background boxes in pure-text fallback mode.
5. **Do not modify user terminal configs:** Never edit `~/.config/foot/foot.ini`, `~/.config/ghostty/config`, or Omarchy theme files unless the user explicitly asks.
6. **Atlas glyph safety:** Any glyph used in `src/scenes_hd.rs` **must** exist in `src/font_atlas.rs::glyph_bitmap`; unknown characters render blank. Never use `✦` or `✧` (missing in `JetBrainsMono Nerd Font`). Preserve the hand-tuned crisp `=` bitmap and geometric `▓`/`▒`/`░` stipples in `src/font_atlas.rs`.
7. **Always read [PITFALLS.md](PITFALLS.md)** before changing PTY I/O, signal handling, Sixel/Kitty escape sequences, resize logic, or daemon lifecycle.
