# Pitfalls & Lessons Learned

Hard-won terminal, PTY, graphics protocol, and rendering gotchas encountered while building `greeting`. Read this before modifying `src/gfx.rs`, `src/scenes.rs`, `src/scenes_hd.rs`, or `src/font_atlas.rs`.

---

## 1. PTY & Process Lifecycle

### Never set `O_NONBLOCK` on `STDIN_FILENO`
In a PTY, `STDIN_FILENO` and `STDOUT_FILENO` point to the **same underlying open file description**. Calling `fcntl(STDIN_FILENO, F_SETFL, O_NONBLOCK)` silently makes `STDOUT_FILENO` non-blocking too, causing large ANSI/Kitty/Sixel frames to fail mid-write with `ErrorKind::WouldBlock` and freeze/corrupt the terminal.
- **Fix:** Use `libc::poll()` with `0` timeout for non-blocking key reads (`poll_key`), and loop on `Interrupted` / `WouldBlock` in `write_all_retry`.

### Background daemon reparenting breaks `/proc/self` ancestor detection
When `greeting` spawns `greeting --daemon` and immediately exits, the daemon is reparented to PID 1 (`systemd`/`init`). Walking `/proc/<self>/stat` parents from inside the daemon will **never** find `foot` or `ghostty`.
- **Fix:** Pass `--shell-pid <ppid>` to `--daemon` and walk `/proc/<shell_pid>/comm` ancestors from `shell_pid` (which remains a direct child of the terminal emulator).

### `tcgetpgrp` alone does not stop the daemon on shell builtins
Checking `libc::tcgetpgrp(STDOUT_FILENO) != shell_pgid` stops the daemon immediately when the user runs external commands (`ls`, `git`, `vim`), because bash gives the foreground process group to the child job. However, shell builtins (`cd`, `echo`, `export`, empty Enter) never change `tcgetpgrp`.
- **Fix:** Pair `tcgetpgrp` monitoring with the armed `PROMPT_COMMAND` hook (`_greeting_prompt_hook`) writing `<pid> 0` → `<pid> 1` → `SIGTERM` on the next prompt.

### Background writes without `SIGTTOU` ignored will suspend the daemon
A background process group writing to a controlling TTY with `TOSTOP` enabled receives `SIGTTOU` and stops.
- **Fix:** Put the daemon in its own process group (`setpgid(0, 0)`) and ignore `SIGTTOU`, `SIGTTIN`, and `SIGINT`, while catching `SIGTERM`, `SIGHUP`, and `SIGWINCH`.

---

## 2. Terminal Graphics Protocols

### `foot` sets `TERM=xterm-256color` and uses Sixel, not Kitty Graphics
On Omarchy/Linux, `foot.ini` commonly overrides `term=xterm-256color` for SSH compatibility, and `foot` implements **Sixel** rather than the Kitty Graphics Protocol. Environment-variable-only detection (`TERM`, `TERM_PROGRAM`, `GHOSTTY_RESOURCES_DIR`) will completely miss `foot` and silently fall back to plain text.
- **Fix:** Check env vars first (skipping graphics inside `TMUX` / `STY`), then walk `/proc/<pid>/comm` ancestors up to 10 hops to detect `ghostty`, `kitty`, `wezterm*` (`Kitty`) and `foot`, `footclient`, `mlterm`, `contour` (`Sixel`).

### Kitty Graphics Protocol: `q=2`, `C=1`, and fixed `i`/`p` are mandatory
- **`q=2`:** Without `q=2` (suppress all responses), Ghostty/Kitty reply `\x1b_Gi=4201;OK\x1b\\` to stdin — which dumps garbage directly into the user's bash prompt!
- **`C=1`:** Prevents the terminal from advancing the cursor past the image placement.
- **`i=4201,p=1`:** Reusing a fixed image ID + placement ID replaces the image in place inside `\x1b[?2026h...\x1b[?2026l` (Synchronized Output) with zero flicker.
- **On startup and resize:** Explicitly emit `\x1b_Ga=d,d=i,i=4201,q=2\x1b\\` before `\x1b[2J` so stale graphics from prior runs or old window sizes are deleted cleanly.

### Never emit `\x1b[K` on left HUD rows in hybrid mode
Erasing to end-of-line (`\x1b[K`) from column 1 sweeps across columns `24+`, which can clear or slice right-hand Kitty/Sixel graphics placements on those rows.
- **Fix:** Pad every left-HUD row (`encode_hd_hud_row`) to exact fixed width `HUD_WIDTH` (`23` cols) with spaces and never emit `\x1b[K` from the HUD encoder.

### Sixel in `foot`: clearing previous transparent frames without ghosting
With Sixel `P2=1` (transparent background), drawing a new frame on top of an old frame without erasing the cells underneath leaves stale pixels from earlier animation frames.
- **Fix:** Inside `\x1b[?2026h...\x1b[?2026l`, emit `\x1b[{row};{img_col}H\x1b[K` across `1..=banner_rows` starting at `img_col` (never col 1!) right before emitting the new Sixel stream. `foot` deletes the old Sixel slice and paints the new one atomically in the same compositor frame.

### Sixel in `foot`: transient `TIOCGWINSZ` races & nearest-neighbor scaling
When a tiling WM (e.g., Hyprland) opens and immediately retiles `foot`, `TIOCGWINSZ` can transiently report mismatched `cols`/`rows` vs `ws_xpixel`/`ws_ypixel`, producing absurd `cell_h` values that cause Frame 1 to emit a giant Sixel image extending past row 15 — leaving an un-erased bottom half stranded below the banner! Additionally, upscaling a `462×330` glyph bitmap with nearest-neighbor creates uneven blurry strokes.
- **Fix:**
  1. Sanity-check `(cell_w, cell_h)` and require `1.25 <= cell_h / cell_w <= 3.2`, falling back to `(11.0, 24.0)` if invalid.
  2. Cap Sixel `scale <= 1.0` so Sixel renders at exact **1:1 native `462×330` pixel sharpness** whenever it fits, only downscaling on very narrow/short panes.

### Sixel color quantization: why fixed 6×6×6 RGB cubes fail on bloom
A uniform 6×6×6 cube (`[0, 20, 40, 60, 80, 100]%`) jumps from `0%` straight to `20%`, making subtle outer bloom noisy. Even a gamma-curved 6-level cube (`[0, 8, 22, 44, 72, 100]%`) breaks hue ratios: dim orange fire bloom `(25, 12, 4)` snaps both R and G to `8%` (`(20, 20, 0)`), turning warm fire halos **olive-green**, and snaps vermilion red highlights into orange blotches.
- **Fix:** Bucket opaque/dithered pixels into a 12-bit (`4×4×4`, 4096-entry) stack histogram, take the **exact average RGB** of each active bucket, cap at 240 Sixel palette entries (mapping any rare overflow to nearest RGB), and feather soft outer bloom alpha with 4×4 Bayer ordered dithering (`af.powf(0.78)`). Preserves true warm/cool hues with zero green shift or blotches in `< 2ms`.

---

## 3. Font Atlas, Bloom & Pure-Text Fallback

### Tiny antialiased `=` collapses into a solid square `▪`
At `9.5px` in `JetBrainsMono`, Cairo's vertical antialiasing straddles half-pixels on `=` (`0x3d`), merging its two horizontal bars into a fuzzy dim 4×4 square across belts, page edges, and water lines.
- **Fix:** Manually set `=` in `src/font_atlas.rs` to two crisp horizontal bars (`row 4` and `row 7`) separated by a clean 2px empty gap (`rows 5–6`).

### Ghostty overrides `▓` (`U+2593`) in plain text
Ghostty's built-in box-drawing renderer replaces `▓` with a flat translucent rectangle instead of a stipple mesh, flattening obsidian/stone textures in raw text mode.
- **Fix:** In HD mode, `src/font_atlas.rs` rasterizes `▓`, `▒`, and `░` as crisp geometric stipple patterns directly into the baked alpha atlas.

### Avoid missing-glyph tofu boxes (`✦`, `✧`)
`JetBrainsMono Nerd Font` lacks `✦` and `✧` in many builds, rendering tofu boxes.
- **Fix:** Stick exclusively to glyphs verified present in `font_atlas.rs` and standard Nerd Fonts: `*`, `·`, `°`, `§`, `◈`, `◇`, `─`, `│`, `═`, `║`, `╬`, `█`, `▓`, `▒`, `░`, `▄`, `▀`, and printable ASCII.

### Do not simulate bloom with ANSI `\x1b[48;2;...m` background cells
Setting per-cell background colors in plain text produces clunky rectangular boxes behind runes and edges. Pure-text fallback must remain foreground-only (`\x1b[38;2;...m`).

### HDR bloom clipping & outer halo cutoff
- **Clipping:** Uncompressed additive bloom (`fg + tight + wide`) clips dense glowing areas (portal centers, stacked solid blocks) to flat white. Apply soft-knee compression (`bloom / (1.0 + max_c * 0.65)`) so neon halos stay richly saturated.
- **Outer ring artifact:** Cutting off transparent RGBA alpha at `out_a < 0.012` creates a faint visible contour around wide bloom; lowering the cutoff to `0.004` (`~1/255`) lets halos dissolve invisibly.

---

## 4. Resize Handling & Testing

### Preventing ghost wrapped lines on terminal resize
If a terminal shrinks narrower than the banner while the daemon writes lines, auto-wrap folds rows onto extra lines and corrupts the prompt below.
- **Fix:** Wrap every frame in `\x1b[?2026h` (sync output) and `\x1b[?7l` / `\x1b[?7h` (disable/re-enable auto-wrap), address rows explicitly (`\x1b[{row};1H`), debounce `SIGWINCH` / dimension changes for `80ms`, clear (`emit_kitty_delete` + `\x1b[2J`), repaint, park cursor at `banner_rows + 2`, and send `SIGWINCH` to `shell_pid` so readline redraws the prompt cleanly.

### Always test headlessly — never pop up GUI terminal windows
Spawning `foot` or `ghostty` during automated verification steals focus and flashes windows on the user's desktop.
- **Fix:**
  - Use `greeting --scene <name> --dump-rgba /tmp/frame.rgba` and composite with Python `cairo` to visually inspect all 6 HD bloom frames.
  - Use Python `pty.openpty()` + `TIOCSWINSZ` (`TERM=foot` or `GHOSTTY_RESOURCES_DIR=/usr/share/ghostty`) to capture and decode raw Sixel frames, verify Kitty Graphics headers (`C=1, q=2, s=462, v=330`), and test live `bash -i` boot, typing, resize, and command-exit lifecycles completely headlessly.
