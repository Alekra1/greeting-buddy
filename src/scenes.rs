use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use rand::Rng;

use crate::gfx::{self, BloomRenderer, GraphicsProto, HdGrid};
use crate::scenes_hd::{render_hd_scene, scene_hud_style};
use crate::sysinfo::SysInfo;

pub const COLS: usize = 70;
pub const ROWS: usize = 17;

const FRAME_MS: u64 = 50; // ~20 FPS for pure-text fallback
const HD_FRAME_MS: u64 = 60; // ~16.6 FPS for HD Kitty Graphics bloom
const HD_BANNER_ROWS: usize = 15;
const HUD_WIDTH: usize = 23;

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
static RESIZED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_stop_signal(_sig: libc::c_int) {
    STOP_REQUESTED.store(true, Ordering::SeqCst);
}

extern "C" fn handle_winch_signal(_sig: libc::c_int) {
    RESIZED.store(true, Ordering::SeqCst);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scene {
    Eclipse,
    Bonfire,
    Torii,
    Portal,
    Grimoire,
    Nether,
}

impl Scene {
    pub const ALL: [Scene; 6] = [
        Scene::Eclipse,
        Scene::Bonfire,
        Scene::Torii,
        Scene::Portal,
        Scene::Grimoire,
        Scene::Nether,
    ];

    pub fn from_name(s: &str) -> Option<Scene> {
        match s.to_ascii_lowercase().as_str() {
            "1" | "eclipse" | "knight" => Some(Scene::Eclipse),
            "2" | "bonfire" | "fire" => Some(Scene::Bonfire),
            "3" | "torii" | "shrine" | "moon" => Some(Scene::Torii),
            "4" | "portal" | "astral" | "gate" => Some(Scene::Portal),
            "5" | "grimoire" | "book" | "tome" => Some(Scene::Grimoire),
            "6" | "nether" | "nether-portal" | "mc" | "minecraft" => Some(Scene::Nether),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Scene::Eclipse => 0,
            Scene::Bonfire => 1,
            Scene::Torii => 2,
            Scene::Portal => 3,
            Scene::Grimoire => 4,
            Scene::Nether => 5,
        }
    }

    pub fn prev(self) -> Scene {
        Scene::ALL[(self.index() + Scene::ALL.len() - 1) % Scene::ALL.len()]
    }

    pub fn random<R: Rng>(rng: &mut R) -> Scene {
        Scene::ALL[rng.gen_range(0..Scene::ALL.len())]
    }

    pub fn random_other<R: Rng>(self, rng: &mut R) -> Scene {
        let mut pick = Scene::random(rng);
        while pick == self {
            pick = Scene::random(rng);
        }
        pick
    }
}

#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    r: u8,
    g: u8,
    b: u8,
    bright: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            r: 0,
            g: 0,
            b: 0,
            bright: false,
        }
    }
}

struct Grid {
    cells: [Cell; COLS * ROWS],
}

impl Grid {
    fn new() -> Self {
        Self {
            cells: [Cell::default(); COLS * ROWS],
        }
    }

    fn clear(&mut self) {
        self.cells.fill(Cell::default());
    }

    #[inline]
    fn set(&mut self, x: isize, y: isize, ch: char, r: f32, g: f32, b: f32, glow: f32) {
        if x < 0 || x >= COLS as isize || y < 0 || y >= ROWS as isize {
            return;
        }
        self.cells[y as usize * COLS + x as usize] = Cell {
            ch,
            r: r.clamp(0.0, 255.0) as u8,
            g: g.clamp(0.0, 255.0) as u8,
            b: b.clamp(0.0, 255.0) as u8,
            bright: glow > 0.62,
        };
    }

    fn put_text(&mut self, x: isize, y: isize, text: &str, r: f32, g: f32, b: f32, glow: f32) {
        for (i, ch) in text.chars().enumerate() {
            if ch == ' ' {
                self.set(x + i as isize, y, ' ', 0.0, 0.0, 0.0, 0.0);
            } else {
                self.set(x + i as isize, y, ch, r, g, b, glow);
            }
        }
    }

    fn encode_row(&self, out: &mut String, y: usize, visible_cols: usize) {
        use std::fmt::Write as _;

        let row_start = y * COLS;
        let mut last_non_space = 0usize;
        for x in 0..visible_cols {
            if self.cells[row_start + x].ch != ' ' {
                last_non_space = x + 1;
            }
        }

        let mut cur_rgb: Option<(u8, u8, u8, bool)> = None;
        for x in 0..last_non_space {
            let c = self.cells[row_start + x];
            if c.ch == ' ' {
                if cur_rgb.is_some() {
                    out.push_str("\x1b[0m");
                    cur_rgb = None;
                }
                out.push(' ');
            } else {
                let key = (c.r, c.g, c.b, c.bright);
                if cur_rgb != Some(key) {
                    if c.bright {
                        let _ = write!(out, "\x1b[1;38;2;{};{};{}m", c.r, c.g, c.b);
                    } else {
                        let _ = write!(out, "\x1b[0;38;2;{};{};{}m", c.r, c.g, c.b);
                    }
                    cur_rgb = Some(key);
                }
                out.push(c.ch);
            }
        }
        if cur_rgb.is_some() {
            out.push_str("\x1b[0m");
        }
        out.push_str("\x1b[K");
    }

    fn encode_ansi(&self, out: &mut String, max_cols: usize, max_rows: usize, trailing_newline: bool) {
        let visible_cols = COLS.min(max_cols);
        let visible_rows = ROWS.min(max_rows);
        for y in 0..visible_rows {
            self.encode_row(out, y, visible_cols);
            if y + 1 < visible_rows || trailing_newline {
                out.push('\n');
            }
        }
    }

    fn encode_ansi_pinned(&self, out: &mut String, max_cols: usize, max_rows: usize) {
        use std::fmt::Write as _;

        let visible_cols = COLS.min(max_cols);
        let visible_rows = ROWS.min(max_rows);
        for y in 0..visible_rows {
            let _ = write!(out, "\x1b[{};1H", y + 1);
            self.encode_row(out, y, visible_cols);
        }
    }
}

// ── Math, Noise & HUD helpers ────────────────────────────────────────────────

#[inline]
fn hash(x: f32, y: f32) -> f32 {
    let n = (x * 127.1 + y * 311.7).sin() * 43758.5453123;
    n - n.floor()
}

fn vnoise(x: f32, y: f32) -> f32 {
    let ix = x.floor();
    let iy = y.floor();
    let fx = x - ix;
    let fy = y - iy;
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let a = hash(ix, iy);
    let b = hash(ix + 1.0, iy);
    let c = hash(ix, iy + 1.0);
    let d = hash(ix + 1.0, iy + 1.0);
    a + (b - a) * ux + (c - a) * uy + (a - b - c + d) * ux * uy
}

fn fbm(x: f32, y: f32) -> f32 {
    vnoise(x, y) * 0.6
        + vnoise(x * 2.1 + 13.7, y * 2.1 + 7.3) * 0.3
        + vnoise(x * 4.3, y * 4.3) * 0.1
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn fire_color(v: f32) -> (f32, f32, f32) {
    let v = v.clamp(0.0, 1.0);
    if v < 0.32 {
        let k = v / 0.32;
        (lerp(80.0, 200.0, k), lerp(18.0, 44.0, k), lerp(8.0, 12.0, k))
    } else if v < 0.66 {
        let k = (v - 0.32) / 0.34;
        (lerp(200.0, 255.0, k), lerp(44.0, 158.0, k), lerp(12.0, 36.0, k))
    } else {
        let k = (v - 0.66) / 0.34;
        (255.0, lerp(158.0, 248.0, k), lerp(36.0, 200.0, k))
    }
}

fn pick_char(ramp: &[u8], v: f32) -> char {
    let idx = (v.clamp(0.0, 0.9999) * ramp.len() as f32) as usize;
    ramp[idx] as char
}

fn trunc(s: &str, max_len: usize) -> String {
    let count = s.chars().count();
    if count <= max_len {
        s.to_string()
    } else if max_len > 1 {
        let mut out: String = s.chars().take(max_len - 1).collect();
        out.push('…');
        out
    } else {
        s.chars().take(max_len).collect()
    }
}

fn username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

fn draw_hud(
    g: &mut Grid,
    title: &str,
    title_rgb: (f32, f32, f32),
    label_rgb: (f32, f32, f32),
    si: &SysInfo,
    user: &str,
) {
    g.put_text(1, 3, title, title_rgb.0, title_rgb.1, title_rgb.2, 0.78);
    g.put_text(
        1,
        4,
        "─────────────────────",
        label_rgb.0 * 0.62,
        label_rgb.1 * 0.62,
        label_rgb.2 * 0.62,
        0.0,
    );
    let rows = [
        ("user", trunc(user, 16)),
        ("os  ", trunc(&si.os, 16)),
        ("kern", trunc(&si.kernel, 16)),
        ("ip  ", trunc(&si.local_ip, 16)),
        ("pkgs", trunc(&si.packages, 16)),
    ];
    for (i, (k, v)) in rows.iter().enumerate() {
        g.put_text(1, 6 + i as isize, k, label_rgb.0, label_rgb.1, label_rgb.2, 0.0);
        g.put_text(6, 6 + i as isize, v, 228.0, 234.0, 244.0, 0.0);
    }
}

// ── Scene 1: Eclipse Knight (High-Detail 70x17 Armored Figure & Flame Blade) ─
// Anchored at x=42, y=5 (22 cols wide, 12 rows tall)
//   P = flared layered pauldrons     C = sculpted steel cuirass
//   R = breastplate crest & ribs     A = articulated vambrace / gauntlet
//   S = chainmail joint shadow       M = tattered flowing midnight cape
//   G = ornate gold belt & brocade   T = royal indigo pleated tabard
//   K = armored knee cop / poleyn    B = steel greaves & tapered sabatons
const ECLIPSE_BODY: &[&str] = &[
    "     PPCCCCCCCCPP     ", // y=5  high steel gorget & shoulder base
    "  PPPPPSCCRRRRCCSPPPP ", // y=6  wide flared layered pauldrons + chest crest
    "  PPAAASCCRRRRCCSMMMP ", // y=7  armored upper arms + sculpted breastplate
    "  PAAA.SCCCPRCCCS.MMM ", // y=8  tapered chest + billowing cape edge
    "   AAA.SCCCCCCCCS.MMM ", // y=9  forearm angled to grip sword hilt
    "   AG..GGGGGGGGGG.MMM ", // y=10 ornate forged gold belt & double-gem buckle
    "      .TTTTGGTTTT.MMM ", // y=11 armored tassets + royal indigo tabard
    "     ..TTTTGGTTTT..MM ", // y=12 pleated skirt + gold brocade stripe
    "     ..TTTTGGTTTT..MM ", // y=13 flowing indigo cloth
    "     ..KKK.GG.KKK..M  ", // y=14 split hem revealing sculpted knee poleyns
    "      .BBB.GG.BBB. M  ", // y=15 articulated steel greaves + cape tail
    "     .BBBB....BBBB.   ", // y=16 sleek tapered armored sabatons on ash
];

fn render_eclipse(g: &mut Grid, t: f32, si: &SysInfo, user: &str) {
    g.clear();
    let body_x0: isize = 42;
    let body_y0: isize = 5;
    let cx: f32 = body_x0 as f32 + 10.5; // 52.5
    let head_y: f32 = 3.0;

    // 1. Ember-dusted ash ground beneath knight's boots (row 16)
    for x in 28..69 {
        let fx = x as f32;
        let d = ((fx - cx) * 0.065).abs();
        let n = hash(fx * 3.1, 16.0);
        if d < 1.8 && n > 0.42 {
            let fade = (1.0 - d / 1.8).clamp(0.0, 1.0);
            let ch = if n > 0.78 { '~' } else if n > 0.56 { '-' } else { '.' };
            g.set(x, 16, ch, 74.0 * fade, 52.0 * fade, 44.0 * fade, 0.0);
        }
    }

    // 2. Drifting glowing ash & embers rising into dark void
    for i in 0..26 {
        let fi = i as f32;
        let sx = (hash(fi, 1.3) * COLS as f32 + (t * 0.8 + fi).sin() * 1.2).rem_euclid(COLS as f32);
        let sy = (hash(fi, 4.7) * ROWS as f32 - t * (0.6 + hash(fi, 9.1) * 1.0)).rem_euclid(ROWS as f32);
        let flicker = 0.42 + 0.58 * (t * 4.5 + fi * 2.3).sin();
        if sx > 24.0 {
            let ch = if flicker > 0.80 {
                '*'
            } else if flicker > 0.52 {
                '·'
            } else {
                '.'
            };
            g.set(
                sx as isize,
                sy as isize,
                ch,
                255.0 * flicker,
                136.0 * flicker,
                48.0 * flicker,
                flicker * 0.65,
            );
        }
    }

    // 3. Detailed Flaming Greatsword (sweeping diagonally from gauntlet (44,10) to tip (24,15))
    let guard_x = 43.0_f32;
    let guard_y = 10.0_f32;
    let tip_x = 24.0_f32;
    let tip_y = 14.8_f32;
    let dx_s = tip_x - guard_x;
    let dy_s = tip_y - guard_y;
    let vx = dx_s;
    let vy = dy_s * 1.85;
    let l2 = vx * vx + vy * vy;

    // Lively multi-layered ASCII fire tongues dancing up from the blade spine
    for y in 6..ROWS {
        for x in 22..45 {
            let fx = x as f32;
            let fy = y as f32;
            let px = fx - guard_x;
            let py = (fy - guard_y) * 1.85;
            let proj = ((px * vx + py * vy) / l2).clamp(0.0, 1.0);
            let bx = guard_x + dx_s * proj;
            let by = guard_y + dy_s * proj;
            let dist_y = fy - by;
            let dist_x = (fx - bx).abs();

            if dist_y < 0.45 && dist_y > -3.1 && dist_x < 1.6 && proj > 0.04 && proj < 0.96 {
                let rise = (1.0 - (-dist_y) / 3.0).clamp(0.0, 1.0);
                let turb = fbm(fx * 0.42 - t * 2.6, fy * 0.58 + t * 4.6);
                let intensity = rise * turb * (1.25 - proj * 0.35);
                if intensity > 0.22 {
                    let v = ((intensity - 0.22) / 0.68).clamp(0.0, 1.0);
                    let (r, gr, b) = fire_color(v);
                    let ch = pick_char(b".,'~^*+&%#@", v);
                    g.set(x as isize, y as isize, ch, r, gr, b, v * 0.92);
                }
            }
        }
    }

    // Crisp molten-steel tapered greatsword blade
    for step in 2..=40 {
        let p = step as f32 / 40.0;
        let bx = guard_x + dx_s * p;
        let by = guard_y + dy_s * p;
        let ix = bx.round() as isize;
        let iy = by.round() as isize;
        let heat = (0.76 + 0.24 * (t * 6.0 - p * 8.0).sin() - p * 0.12).clamp(0.4, 1.0);
        let (fr, fg, fb) = fire_color(heat);
        let ch = if p > 0.94 {
            '<'
        } else if p > 0.78 {
            '-'
        } else if step % 4 == 0 {
            '/'
        } else if step % 2 == 0 {
            '='
        } else {
            '#'
        };
        g.set(
            ix,
            iy,
            ch,
            lerp(240.0, fr, 0.52),
            lerp(222.0, fg, 0.52),
            lerp(196.0, fb, 0.52),
            0.94,
        );
    }

    // Flared crossguard, leather grip in gauntlet, and forged gold pommel
    let hilt_pts: &[(isize, isize, char, (f32, f32, f32), f32)] = &[
        (43, 9, '\\', (232.0, 202.0, 136.0), 0.75),
        (43, 10, 'X', (252.0, 220.0, 152.0), 0.90),
        (43, 11, '/', (222.0, 192.0, 124.0), 0.70),
        (44, 10, '=', (186.0, 146.0, 98.0), 0.25),
        (46, 10, 'o', (238.0, 198.0, 116.0), 0.75),
    ];
    for &(hx, hy, ch, (hr, hg, hb), glow) in hilt_pts {
        g.set(hx, hy, ch, hr, hg, hb, glow);
    }

    // 4. High-Detail Sculpted Armored Knight Body
    let steel_ramp = b".:-=+*#%&$8@";
    let cloth_ramp = b".:;=+%#&";
    for (ry, row_str) in ECLIPSE_BODY.iter().enumerate() {
        let y = body_y0 + ry as isize;
        let fy = y as f32;
        for (rx, mask) in row_str.chars().enumerate() {
            if mask == ' ' || mask == '.' {
                continue;
            }
            let x = body_x0 + rx as isize;
            let fx = x as f32;
            let dx = fx - cx;
            let nx = (dx / 9.2).clamp(-1.0, 1.0);
            let nz = (1.0 - nx * nx * 0.75).max(0.1).sqrt();
            let tex = hash(fx * 3.7, fy * 7.1);

            let top_glow = (1.0 - (fy - 4.5) / 5.4).clamp(0.0, 1.0) * (0.45 + 0.55 * nz);
            let sword_dist = ((fx - 42.0) * 0.12).hypot((fy - 10.5) * 0.21);
            let sword_light = (1.0 - sword_dist).clamp(0.0, 1.0)
                * (-nx * 0.85 + 0.45).clamp(0.0, 1.0)
                * (0.86 + 0.14 * (t * 7.0 + fy).sin());

            match mask {
                'P' => {
                    // Flared layered pauldrons with crisp outer steel rim
                    let edge = (dx.abs() - 5.2).clamp(0.0, 3.6) / 3.6;
                    let lum = (0.36 + nz * 0.42 + edge * 0.26 + top_glow * 0.6).clamp(0.16, 1.0);
                    let warm = (top_glow * 0.9 + sword_light * 0.8).clamp(0.0, 1.0);
                    let r = lerp(lerp(86.0, 222.0, lum), 255.0, warm * 0.72);
                    let gr = lerp(lerp(94.0, 228.0, lum), 186.0, warm * 0.70);
                    let b = lerp(lerp(116.0, 244.0, lum), 86.0, warm * 0.65);
                    let ch = if edge > 0.74 {
                        if dx < 0.0 { '/' } else { '\\' }
                    } else if ry == 6 && edge > 0.35 {
                        '='
                    } else {
                        pick_char(steel_ramp, lum * 0.85 + tex * 0.18)
                    };
                    g.set(x, y, ch, r, gr, b, warm * 0.68 + edge * 0.25);
                }
                'R' => {
                    // Ornate central breastplate crest & specular rib ridges
                    let lum = (0.62 + top_glow * 0.48 + sword_light * 0.35).clamp(0.32, 1.0);
                    let warm = (top_glow * 0.75 + sword_light * 0.6).clamp(0.0, 1.0);
                    let r = lerp(196.0 * lum, 255.0, warm * 0.68);
                    let gr = lerp(206.0 * lum, 200.0, warm * 0.65);
                    let b = lerp(228.0 * lum, 110.0, warm * 0.62);
                    let ch = if dx.abs() < 0.6 {
                        '|'
                    } else if ry == 6 && dx.abs() < 1.7 {
                        'V'
                    } else {
                        pick_char(b"=#%8@", lum)
                    };
                    g.set(x, y, ch, r, gr, b, 0.76);
                }
                'C' | 'A' => {
                    let spec = (nz * 0.85 - nx * 0.35).clamp(0.0, 1.0).powi(3);
                    let lum = (0.20 + nz * 0.38 + spec * 0.36 + top_glow * 0.5 + sword_light * 0.5)
                        .clamp(0.10, 1.0);
                    let warm = (top_glow * 0.75 + sword_light * 0.92).clamp(0.0, 1.0);
                    let r = lerp(lerp(58.0, 194.0, lum), 255.0, warm * 0.74);
                    let gr = lerp(lerp(66.0, 202.0, lum), 174.0, warm * 0.70);
                    let b = lerp(lerp(88.0, 222.0, lum), 80.0, warm * 0.66);
                    let ch = pick_char(steel_ramp, lum * 0.82 + tex * 0.22);
                    g.set(x, y, ch, r, gr, b, warm * 0.55);
                }
                'S' => {
                    let ch = if tex > 0.5 { ':' } else { ';' };
                    g.set(
                        x,
                        y,
                        ch,
                        50.0 + sword_light * 90.0,
                        56.0 + sword_light * 52.0,
                        76.0 + sword_light * 24.0,
                        0.0,
                    );
                }
                'M' => {
                    // Flowing tattered midnight mantle swaying subtly in heat drafts
                    let sway = ((t * 2.5 - fy * 0.45).sin() * 0.4).round() as isize;
                    let shade = (0.24 + tex * 0.28 + top_glow * 0.25).clamp(0.12, 0.72);
                    let ch = if ry >= 13 {
                        if tex > 0.5 { '/' } else { ';' }
                    } else {
                        pick_char(b":;/|\\", shade)
                    };
                    g.set(
                        x + if ry >= 11 { sway } else { 0 },
                        y,
                        ch,
                        44.0 * shade + 22.0,
                        48.0 * shade + 24.0,
                        82.0 * shade + 38.0,
                        0.0,
                    );
                }
                'G' => {
                    let shade = (0.50 + nz * 0.45 + sword_light * 0.4).clamp(0.28, 1.0);
                    let ch = if y == 10 {
                        if dx.abs() < 1.1 { '◈' } else { '=' }
                    } else if tex > 0.5 {
                        '&'
                    } else {
                        '%'
                    };
                    g.set(
                        x,
                        y,
                        ch,
                        lerp(122.0, 252.0, shade),
                        lerp(88.0, 204.0, shade),
                        lerp(28.0, 86.0, shade),
                        0.72,
                    );
                }
                'T' => {
                    let fold = 0.72 + 0.28 * (dx * 1.85 + fy * 0.25).sin();
                    let shade = ((0.30 + nz * 0.55) * fold + sword_light * 0.35).clamp(0.14, 1.0);
                    let r = lerp(28.0, 72.0, shade) + sword_light * 98.0;
                    let gr = lerp(34.0, 88.0, shade) + sword_light * 44.0;
                    let b = lerp(82.0, 192.0, shade) + sword_light * 14.0;
                    let ch = pick_char(cloth_ramp, shade * 0.82 + tex * 0.2);
                    g.set(x, y, ch, r, gr, b, 0.0);
                }
                'K' => {
                    // Articulated knee poleyns (armored knee guards)
                    let ch = match rx {
                        7 | 14 => '<',
                        8 | 15 => '#',
                        _ => '>',
                    };
                    let warm = (sword_light * 0.72).clamp(0.0, 1.0);
                    g.set(
                        x,
                        y,
                        ch,
                        lerp(178.0, 250.0, warm * 0.65),
                        lerp(188.0, 178.0, warm * 0.60),
                        lerp(214.0, 96.0, warm * 0.55),
                        0.72,
                    );
                }
                'B' => {
                    // Sculpted steel greaves & tapered armored sabatons
                    let local_cx = if dx < 0.0 { cx - 3.2 } else { cx + 3.2 };
                    let lnx = ((fx - local_cx) / 2.1).clamp(-1.0, 1.0);
                    let lnz = (1.0 - lnx * lnx).max(0.15).sqrt();
                    let lum = (0.30 + lnz * 0.50 + sword_light * 0.42).clamp(0.16, 1.0);
                    let warm = (sword_light * 0.72).clamp(0.0, 1.0);
                    let r = lerp(lerp(72.0, 202.0, lum), 248.0, warm * 0.65);
                    let gr = lerp(lerp(80.0, 212.0, lum), 166.0, warm * 0.60);
                    let b = lerp(lerp(100.0, 232.0, lum), 80.0, warm * 0.55);
                    let ch = if y == 16 {
                        if dx < 0.0 {
                            match rx {
                                6 => '/',
                                7 => '=',
                                8 => '#',
                                _ => '\\',
                            }
                        } else {
                            match rx {
                                14 => '/',
                                15 => '#',
                                16 => '=',
                                _ => '\\',
                            }
                        }
                    } else if lnz > 0.85 {
                        '#'
                    } else {
                        pick_char(b":=+%#", lum)
                    };
                    g.set(x, y, ch, r, gr, b, if lnz > 0.85 { 0.68 } else { 0.2 });
                }
                _ => {}
            }
        }
    }

    // 5. Blazing Solar-Eclipse Halo Head (rich swirling corona around pitch void)
    for y in 0..=6 {
        for x in (cx as isize - 9)..=(cx as isize + 9) {
            let fx = x as f32;
            let fy = y as f32;
            let dx = (fx - cx) * 0.56;
            let dy = fy - head_y;
            let r = dx.hypot(dy);
            let ang = dy.atan2(dx);
            let wave = (ang * 5.0 + t * 3.1).sin() * 0.25 + (ang * 8.0 - t * 4.3).cos() * 0.16;
            let top_lift = if dy < 0.0 {
                -dy * 0.26 * fbm(fx * 0.46, fy * 0.52 + t * 3.4)
            } else {
                0.0
            };
            let ring_dist = (r - (2.15 + wave * 0.50)).abs();

            if r < 1.48 {
                g.set(x, y as isize, ' ', 0.0, 0.0, 0.0, 0.0);
                if r > 1.12 && hash(fx, fy) > 0.70 {
                    g.set(x, y as isize, ':', 112.0, 38.0, 14.0, 0.2);
                }
            } else if ring_dist < 1.08 + top_lift {
                let core = (1.0 - ring_dist / (1.08 + top_lift)).clamp(0.0, 1.0);
                let noise = fbm(fx * 0.44 + t.cos() * 0.5, fy * 0.56 + t * 3.8);
                let v = (core * (0.72 + noise * 0.58)).clamp(0.0, 1.0);
                if v > 0.16 {
                    let (cr, cg, cb) = fire_color(v);
                    let ch = pick_char(b".'~^*+%#8@", v);
                    g.set(x, y as isize, ch, cr, cg, cb, v);
                }
            }
        }
    }

    draw_hud(g, "◈ ECLIPSE KNIGHT", (214.0, 178.0, 108.0), (115.0, 126.0, 150.0), si, user);
}

// ── Scene 2: Ruined Bonfire (Detailed 70x17 Gothic Sanctuary) ────────────────

fn render_bonfire(g: &mut Grid, t: f32, si: &SysInfo, user: &str) {
    g.clear();
    let fx = 49.0_f32;
    let fy = 12.2_f32;
    let pulse = 0.88 + 0.12 * (t * 7.5).sin() + 0.06 * (t * 13.1).cos();

    for y in 0..8 {
        for x in 24..COLS {
            if hash(x as f32 * 7.0, y as f32 * 13.0) > 0.986 {
                let tw = 0.5 + 0.5 * (t * 2.2 + x as f32).sin();
                g.set(
                    x as isize,
                    y as isize,
                    if tw > 0.72 { '*' } else { '·' },
                    148.0 * tw,
                    168.0 * tw,
                    208.0 * tw,
                    0.25,
                );
            }
        }
    }

    // Ruined gothic sanctuary pillars & soaring broken arch
    let stone_glyphs = b":;=#%H8";
    for y in 1..=13 {
        for x in 29..=67 {
            let x_f = x as f32;
            let y_f = y as f32;
            let is_left = (32..=36).contains(&x) && y >= 4;
            let is_right = (61..=65).contains(&x) && y >= 2;
            let is_spire = (56..=58).contains(&x) && (3..=13).contains(&y);
            let arch_dist = (((x_f - 42.5) * 0.70).hypot(y_f - 7.4) - 6.6).abs();
            let is_arch = arch_dist < 0.92 && y_f < 7.4 && (32..=48).contains(&x);

            if is_left || is_right || is_spire || is_arch {
                let d_fire = ((x_f - fx) * 0.62).hypot((y_f - fy) * 1.10);
                let warm = (1.0 - d_fire / 16.0).clamp(0.0, 1.0) * pulse;
                let n = hash(x_f * 3.0, y_f * 5.0);
                let ch = pick_char(stone_glyphs, 0.35 + warm * 0.5 + n * 0.2);
                g.set(
                    x as isize,
                    y as isize,
                    ch,
                    lerp(36.0, 220.0, warm),
                    lerp(44.0, 122.0, warm),
                    lerp(66.0, 56.0, warm),
                    warm * 0.45,
                );
            }
        }
    }

    // Cobblestone floor bathed in live radial warm firelight (rows 13..16)
    let floor_glyphs = b" .,:;-=~#%";
    for y in 13..ROWS {
        for x in 0..COLS {
            let x_f = x as f32;
            let y_f = y as f32;
            let d_fire = ((x_f - fx) * 0.54).hypot((y_f - fy) * 1.50);
            let light = (1.0 - d_fire / 19.5).clamp(0.0, 1.0) * pulse;
            let n = hash(x_f * 5.0, y_f * 11.0);
            if light > 0.05 || n > 0.68 {
                let ch = pick_char(floor_glyphs, light * 0.75 + n * 0.28);
                g.set(
                    x as isize,
                    y as isize,
                    ch,
                    lerp(28.0, 232.0, light),
                    lerp(32.0, 126.0, light),
                    lerp(46.0, 50.0, light),
                    light * 0.35,
                );
            }
        }
    }

    // Ash & glowing ember mound
    for y in 12..=14 {
        let w = (y - 11) as f32 * 3.4;
        for x in (fx - w).floor() as isize..=(fx + w).ceil() as isize {
            let n = hash(x as f32, y as f32);
            let hot = (1.0 - (x as f32 - fx).abs() / (w + 1.0)).clamp(0.0, 1.0) * pulse;
            let ch = if n > 0.55 {
                '&'
            } else if n > 0.25 {
                '%'
            } else {
                '#'
            };
            g.set(
                x,
                y as isize,
                ch,
                lerp(78.0, 240.0, hot),
                lerp(44.0, 118.0, hot),
                lerp(36.0, 44.0, hot),
                hot * 0.45,
            );
        }
    }

    // Volumetric rising bonfire flame
    let flame_glyphs = b".'~^*+S&%#8@";
    for y in 3..=12 {
        let h = (12 - y) as f32 / 9.0;
        let sway = (t * 4.2 - h * 4.8).sin() * (h * 1.9);
        let radius = (1.0 - h * 0.78) * 5.1 + 0.55;
        for x in (fx as isize - 7)..=(fx as isize + 7) {
            let dx = (x as f32 - (fx + sway)).abs();
            if dx > radius + 1.3 {
                continue;
            }
            let edge = (1.0 - dx / radius).clamp(0.0, 1.0);
            let turb = fbm(x as f32 * 0.44, y as f32 * 0.50 + t * 4.8);
            let v = ((edge * 0.95 + turb * 0.42 - h * 0.44) * pulse).clamp(0.0, 1.0);
            if v > 0.18 {
                let (r, gr, b) = fire_color(((v - 0.18) / 0.75).clamp(0.0, 1.0));
                let ch = pick_char(flame_glyphs, v);
                g.set(x, y as isize, ch, r, gr, b, v);
            }
        }
    }

    // Coiled greatsword thrust into the fire
    let fxi = fx as isize;
    let sword = [
        (fxi + 2, 5, '▄'),
        (fxi + 2, 6, '║'),
        (fxi + 1, 7, '═'),
        (fxi + 2, 7, '╬'),
        (fxi + 3, 7, '═'),
        (fxi + 1, 8, '/'),
        (fxi + 1, 9, 'S'),
        (fxi, 10, '/'),
        (fxi, 11, 'S'),
        (fxi - 1, 12, '│'),
    ];
    for (sx, sy, sch) in sword {
        let heat = ((sy as f32 - 6.0) / 6.0).clamp(0.0, 1.0);
        g.set(
            sx,
            sy,
            sch,
            lerp(188.0, 255.0, heat),
            lerp(196.0, 224.0, heat),
            lerp(212.0, 134.0, heat),
            0.82,
        );
    }

    // Floating night embers
    for i in 0..20 {
        let fi = i as f32;
        let p = (t * (0.28 + hash(fi, 1.0) * 0.25) + hash(fi, 2.0)).fract();
        let ex = fx + (hash(fi, 3.0) - 0.5) * 16.0 + (t * 2.5 + fi).sin() * 2.0;
        let ey = 12.5 - p * 12.0;
        let glow = (1.0 - p) * (0.6 + 0.4 * (t * 8.0 + fi).sin());
        g.set(
            ex as isize,
            ey as isize,
            if glow > 0.6 { '*' } else { '·' },
            255.0 * glow,
            148.0 * glow,
            46.0 * glow,
            glow * 0.88,
        );
    }

    draw_hud(g, "B O N F I R E   L I T", (238.0, 178.0, 92.0), (138.0, 122.0, 104.0), si, user);
}

// ── Scene 3: Moonlit Torii (Detailed 70x17 Crisp Block Shrine + Blood Moon) ─
// 27 cols wide, exact center col 13:
const TORII_ART: &[&str] = &[
    " ▀▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▀ ", // 0  upturned kasagi roof wings
    "   ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀   ", // 1  heavy underbeam
    "      ██     ▓     ██      ", // 2  pillars + gilded central plaque
    "   ▄▄█████████████████▄▄   ", // 3  wide nuki crossbeam
    "      ██           ██      ", // 4  gate opening framing blood moon
    "      ██           ██      ", // 5
    "      ██           ██      ", // 6
    "      ██           ██      ", // 7
    "     ▄██▄         ▄██▄     ", // 8  carved stone bases in water
];

fn render_torii(g: &mut Grid, t: f32, si: &SysInfo, user: &str) {
    g.clear();
    let tx0: isize = 35;
    let ty0: isize = 4;
    let mcx = 48.0_f32; // exact center (35 + 13)
    let mcy = 6.2_f32;

    // Starry sky
    for y in 0..12 {
        for x in 25..COLS {
            if hash(x as f32 * 7.0, y as f32 * 13.0) > 0.984 {
                let tw = 0.5 + 0.5 * (t * 2.2 + x as f32).sin();
                g.set(
                    x as isize,
                    y as isize,
                    if tw > 0.72 { '*' } else { '·' },
                    184.0 * tw,
                    196.0 * tw,
                    228.0 * tw,
                    0.25,
                );
            }
        }
    }

    // Huge, lushly shaded Blood Moon rising behind the shrine gate (no bg boxes!)
    let moon_ramp = b".:-=+*#%8@";
    for y in 0..13 {
        for x in 30..67 {
            let dx = (x as f32 - mcx) * 0.52;
            let dy = y as f32 - mcy;
            let d = dx.hypot(dy);
            if d < 5.6 {
                let core = (1.0 - d / 5.6).clamp(0.0, 1.0);
                let crater = fbm(x as f32 * 0.34, y as f32 * 0.44);
                let v = (0.30 + core * 0.62 + crater * 0.24).clamp(0.20, 1.0);
                let ch = pick_char(moon_ramp, v);
                // Bright pale rose-gold center -> rich glowing crimson outer disc
                g.set(
                    x as isize,
                    y as isize,
                    ch,
                    lerp(192.0, 255.0, core),
                    lerp(66.0, 212.0, core * core),
                    lerp(76.0, 194.0, core * core),
                    core * 0.90,
                );
            } else if d < 7.3 && hash(x as f32, y as f32) > 0.60 {
                let fade = (1.0 - (d - 5.6) / 1.7).clamp(0.0, 1.0);
                g.set(
                    x as isize,
                    y as isize,
                    '·',
                    155.0 * fade,
                    48.0 * fade,
                    58.0 * fade,
                    0.2,
                );
            }
        }
    }

    // Distant silhouetted pine islands flanking the shrine horizon (rows 10..12)
    let pines: &[(isize, isize)] = &[(28, 11), (31, 12), (65, 11), (68, 12)];
    for &(px, py) in pines {
        g.set(px, py - 1, '^', 54.0, 68.0, 88.0, 0.0);
        g.put_text(px - 1, py, "/#\\", 42.0, 56.0, 76.0, 0.0);
    }

    // Shimmering dark water with crimson-gold moon & twin pillar reflections (rows 13..16)
    for y in 13..ROWS {
        let depth = (y - 13) as f32 / 3.0;
        for x in 24..COLS {
            let fx = x as f32;
            let fy = y as f32;
            let wave = (fx * 0.42 + t * 2.8 + fy * 1.9).sin() * 0.5 + 0.5;
            let d_moon = (fx - mcx + (t * 2.1 + fy * 1.7).sin() * 1.1).abs();
            let moon_ref = (1.0 - d_moon / (6.8 - depth * 1.6)).clamp(0.0, 1.0);
            let pillar_ref = (fx - 41.5).abs() < 1.2 || (fx - 54.5).abs() < 1.2;
            if moon_ref > 0.12 {
                let w = moon_ref * (0.45 + 0.55 * wave);
                let ch = if wave > 0.60 {
                    '~'
                } else if wave > 0.28 {
                    '='
                } else {
                    '-'
                };
                g.set(
                    x as isize,
                    y as isize,
                    ch,
                    lerp(135.0, 255.0, w),
                    lerp(42.0, 154.0, w),
                    lerp(48.0, 138.0, w),
                    moon_ref * 0.80,
                );
            } else if pillar_ref && hash(fx, fy) > 0.28 {
                g.set(x as isize, y as isize, '~', 112.0, 34.0, 42.0, 0.15);
            } else if hash(fx * 3.0, fy * 5.0) > 0.54 {
                g.set(
                    x as isize,
                    y as isize,
                    if wave > 0.5 { '~' } else { '-' },
                    44.0 + wave * 28.0,
                    58.0 + wave * 34.0,
                    88.0 + wave * 42.0,
                    0.0,
                );
            }
        }
    }

    // Drifting glowing sakura petals (drawn behind the Torii silhouette)
    for i in 0..18 {
        let fi = i as f32;
        let px = ((hash(fi, 1.7) * 44.0 + t * (2.6 + hash(fi, 2.3) * 2.0)) % 44.0) + 25.0;
        let py = (hash(fi, 4.1) * ROWS as f32 + t * (0.9 + hash(fi, 5.9) * 0.7) + (t * 2.0 + fi).sin() * 0.5)
            .rem_euclid(ROWS as f32);
        let ch = match i % 3 {
            0 => '*',
            1 => ',',
            _ => '·',
        };
        g.set(px as isize, py as isize, ch, 255.0, 188.0, 204.0, 0.85);
    }

    // Iconic crisp two-tone vermilion Torii gate (with gilded central shrine plaque!)
    for (ry, line) in TORII_ART.iter().enumerate() {
        for (rx, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = tx0 + rx as isize;
            let y = ty0 + ry as isize;
            if ch == '▓' {
                // Gilded shrine tablet at center of upper lintel
                g.set(x, y, '◈', 252.0, 208.0, 118.0, 0.92);
            } else if ry == 8 {
                // Dark carved stone bases standing in the water
                g.set(x, y, ch, 86.0, 74.0, 88.0, 0.15);
            } else {
                let is_bright_rim = ry == 0 || ry == 3 || rx == 6 || rx == 20;
                g.set(
                    x,
                    y,
                    ch,
                    if is_bright_rim { 236.0 } else { 174.0 },
                    if is_bright_rim { 62.0 } else { 34.0 },
                    if is_bright_rim { 56.0 } else { 42.0 },
                    if is_bright_rim { 0.78 } else { 0.2 },
                );
            }
        }
    }

    draw_hud(g, "◈ SHRINE GATE", (238.0, 128.0, 134.0), (148.0, 118.0, 132.0), si, user);
}

// ── Scene 4: Astral Gate (Detailed 70x17 Carved Archway + Multi-Arm Vortex) ─

fn render_portal(g: &mut Grid, t: f32, si: &SysInfo, user: &str) {
    g.clear();
    let cx = 48.5_f32;
    let cy = 7.8_f32;

    // Intricate multi-arm swirling violet-to-cyan-to-white void vortex
    let ramp = b" .:-=+*#%8@";
    for y in 2..14 {
        for x in 38..60 {
            let dx = (x as f32 - cx) * 0.54;
            let dy = y as f32 - cy;
            let r = dx.hypot(dy);
            let inside_arch = if y <= 8 { r < 5.6 } else { true };
            if inside_arch {
                let ang = dy.atan2(dx);
                let swirl1 = (ang * 3.0 + r * 1.45 - t * 3.8).sin() * 0.5 + 0.5;
                let swirl2 = (ang * 5.0 - r * 0.95 + t * 2.2).cos() * 0.5 + 0.5;
                let swirl = swirl1 * 0.72 + swirl2 * 0.28;
                let core = (1.0 - r / 6.0).clamp(0.0, 1.0);
                let v = (0.18 + core * 0.56 + swirl * 0.44).clamp(0.16, 1.0);
                let ch = if r < 1.15 {
                    if v > 0.62 { '@' } else { '◈' }
                } else {
                    pick_char(ramp, v)
                };
                let rc = lerp(128.0, 242.0, core * core) + swirl * 34.0;
                let gc = lerp(74.0, 252.0, (core * 0.8 + swirl * 0.45).clamp(0.0, 1.0));
                let bc = 255.0;
                g.set(x as isize, y as isize, ch, rc, gc, bc, (core + swirl * 0.45).clamp(0.0, 1.0));
            }
        }
    }

    // Ancient carved stone archway: semicircular arch (y<=8) + fluted rune pillars (y=8..14)
    let stone = b":;=#%H8";
    for y in 0..15 {
        for x in 31..67 {
            let dx = (x as f32 - cx) * 0.54;
            let dy = y as f32 - cy;
            let r = dx.hypot(dy);
            let in_ring = y <= 8 && (5.55..=7.30).contains(&r);
            let in_left = (35..=38).contains(&x) && (8..=14).contains(&y);
            let in_right = (59..=62).contains(&x) && (8..=14).contains(&y);
            if in_ring || in_left || in_right {
                let edge_dist = if y <= 8 {
                    (r - 5.6).abs()
                } else {
                    ((x as f32 - 38.0).abs().min((x as f32 - 59.0).abs())) * 0.54
                };
                let inner_rim = (1.0 - edge_dist / 1.55).clamp(0.0, 1.0);
                let n = hash(x as f32 * 3.0, y as f32 * 7.0);
                // Glowing engraved pillar runes on rows 10 & 12 + keystone gem at top center
                let is_keystone = y == 1 && (x == 48 || x == 49);
                let is_pillar_rune = (y == 10 || y == 12) && (x == 36 || x == 60);
                if is_keystone {
                    g.set(x as isize, y as isize, '◈', 188.0, 234.0, 255.0, 0.95);
                } else if is_pillar_rune {
                    let pulse = 0.65 + 0.35 * (t * 3.0 + y as f32).sin();
                    g.set(x as isize, y as isize, '§', 165.0 * pulse + 45.0, 215.0 * pulse + 30.0, 255.0, 0.9);
                } else {
                    let ch = pick_char(stone, 0.32 + inner_rim * 0.45 + n * 0.25);
                    g.set(
                        x as isize,
                        y as isize,
                        ch,
                        lerp(48.0, 182.0, inner_rim),
                        lerp(54.0, 222.0, inner_rim),
                        lerp(80.0, 255.0, inner_rim),
                        inner_rim * 0.78,
                    );
                }
            }
        }
    }

    // Illuminated stepped stone dais below portal (rows 14..16)
    for y in 14..ROWS {
        let w = 15.0 + (y - 14) as f32 * 2.8;
        for x in (cx - w).floor() as isize..=(cx + w).ceil() as isize {
            if x < 26 || x >= COLS as isize {
                continue;
            }
            let d = (x as f32 - cx).abs() / w;
            let glow = (1.0 - d * 0.9).clamp(0.0, 1.0) * (1.0 - (y - 14) as f32 * 0.28);
            let ch = if hash(x as f32 * 5.0, y as f32 * 11.0) > 0.5 { '=' } else { '-' };
            g.set(
                x,
                y as isize,
                ch,
                lerp(46.0, 158.0, glow),
                lerp(56.0, 212.0, glow),
                lerp(84.0, 255.0, glow),
                glow * 0.55,
            );
        }
    }

    // Stardust pulled inward into the portal
    for i in 0..20 {
        let fi = i as f32;
        let a = hash(fi, 1.3) * std::f32::consts::TAU + t * 0.25;
        let p = 1.0 - (t * 0.45 + hash(fi, 2.7)).fract();
        let sx = cx + a.cos() * (5.2 + p * 13.5);
        let sy = cy + a.sin() * (2.8 + p * 6.0);
        if sx > 26.0 && ((sx - cx) * 0.54).hypot(sy - cy) > 7.1 {
            g.set(
                sx as isize,
                sy as isize,
                if p < 0.4 { '*' } else { '·' },
                182.0,
                232.0,
                255.0,
                (1.0 - p) * 0.9,
            );
        }
    }

    draw_hud(g, "◈ ASTRAL GATE", (168.0, 152.0, 255.0), (118.0, 128.0, 168.0), si, user);
}

// ── Scene 5: Arcane Grimoire (Detailed 70x17 Tome + Carved Block Altar) ─────
// 31 cols wide, exact center col 15:
const BOOK_ART: &[&str] = &[
    "   .──────────. .──────────.   ", // 0  top illuminated page curves
    "  /  ~~~~~~~~  V  ~~~~~~~~  \\  ", // 1  glowing arcane script
    " /  ~~~~~~~~~~ | ~~~~~~~~~~  \\ ", // 2  mid-page script & spine
    " |============\\|/============| ", // 3  thick stacked gilded page edges
    "  \\___________\\|/___________/  ", // 4  ornate leather binding
];

const ALTAR_ART: &[&str] = &[
    "      ▄▄███████████████▄▄      ", // 0  weighty carved altar cap (no bg boxes!)
    "       |##%###§#§###%##|       ", // 1  engraved stone frieze
    "     __|##%####◈####%##|__     ", // 2  glowing amethyst sigil center
    "   _/====%#%=======%#%====\\_   ", // 3  stepped stone altar plinth
];

fn render_grimoire(g: &mut Grid, t: f32, si: &SysInfo, user: &str) {
    g.clear();
    let bx0: isize = 33;
    let cx = bx0 as f32 + 15.0; // 48.0
    let hover: isize = if (t * 2.3).sin() > 0.35 { 0 } else { 1 };
    let book_y = 6 + hover;

    // Subtle orbiting arcane motes framing the levitating spellbook
    for i in 0..10 {
        let a = (i as f32 / 10.0) * std::f32::consts::TAU + t * 0.45;
        let rx = cx + a.cos() * 16.5;
        let ry = (book_y as f32 + 1.0) + a.sin() * 4.8;
        if rx > 26.0 && rx < COLS as f32 - 1.0 && ry >= 0.0 && ry < 13.0 {
            let d = (a.sin() + 1.0) * 0.5;
            g.set(
                rx.round() as isize,
                ry.round() as isize,
                if i % 2 == 0 { '·' } else { '°' },
                155.0 + d * 85.0,
                115.0 + d * 95.0,
                220.0 + d * 35.0,
                d * 0.65,
            );
        }
    }

    // Rising twin ribbons of amethyst & gold arcane runes floating cleanly in dark void
    let runes = ['·', '+', '*', '°', '§', '◇', '◈', '^', '~'];
    for y in 0..=book_y {
        let h = (book_y - y) as f32 / 7.5;
        for strand in 0..2 {
            let phase = strand as f32 * std::f32::consts::PI;
            let sx = cx + (y as f32 * 0.62 - t * 3.0 + phase).sin() * (2.8 + h * 4.2);
            for dx in -1_isize..=1 {
                let x = sx.round() as isize + dx;
                let core = if dx == 0 { 1.0 } else { 0.48 };
                let v = ((1.0 - h * 0.46) * core).clamp(0.28, 1.0);
                let ch = if dx == 0 {
                    runes[((y as usize) + (t * 4.0) as usize + strand * 3) % runes.len()]
                } else if hash(x as f32, y as f32) > 0.45 {
                    '·'
                } else {
                    '.'
                };
                if strand == 0 {
                    g.set(
                        x,
                        y,
                        ch,
                        lerp(162.0, 242.0, v),
                        lerp(102.0, 188.0, v),
                        255.0,
                        v * 0.92,
                    );
                } else {
                    g.set(
                        x,
                        y,
                        ch,
                        255.0,
                        lerp(156.0, 232.0, v),
                        lerp(70.0, 152.0, v),
                        v * 0.88,
                    );
                }
            }
        }
    }

    // Detailed levitating open spellbook (curved parchment, shimmering runes, gilded page stack)
    for (ry, line) in BOOK_ART.iter().enumerate() {
        for (rx, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = bx0 + rx as isize;
            let y = book_y + ry as isize;
            if ch == '~' {
                let shimmer = 0.6 + 0.4 * (t * 4.2 + rx as f32 * 0.6).sin();
                g.set(
                    x,
                    y,
                    if shimmer > 0.82 { '=' } else { '~' },
                    224.0 * shimmer + 30.0,
                    178.0 * shimmer + 35.0,
                    255.0,
                    0.82,
                );
            } else if ry == 3 && ch == '=' {
                // Gilded stacked page edges
                g.set(x, y, '=', 225.0, 196.0, 142.0, 0.55);
            } else if ry == 4 {
                // Ornate deep amethyst leather cover
                g.set(x, y, ch, 178.0, 132.0, 218.0, 0.45);
            } else {
                g.set(x, y, ch, 250.0, 232.0, 192.0, 0.80);
            }
        }
    }

    // Carved stone altar pedestal below (rows 13..16) with crisp block top & glowing sigil
    for (ry, line) in ALTAR_ART.iter().enumerate() {
        for (rx, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = bx0 + rx as isize;
            let y = 13 + ry as isize;
            if ch == '◈' {
                g.set(x, y, '◈', 238.0, 192.0, 255.0, 1.0);
            } else if ch == '§' {
                g.set(x, y, '§', 188.0, 148.0, 238.0, 0.72);
            } else {
                let top_light = match ry {
                    0 => 0.88,
                    1 => 0.52,
                    2 => 0.40,
                    _ => 0.30,
                };
                g.set(
                    x,
                    y,
                    ch,
                    lerp(56.0, 156.0, top_light),
                    lerp(54.0, 134.0, top_light),
                    lerp(86.0, 212.0, top_light),
                    if ry == 0 { 0.65 } else { 0.15 },
                );
            }
        }
    }

    draw_hud(g, "◈ ARCANE GRIMOIRE", (204.0, 152.0, 255.0), (134.0, 118.0, 162.0), si, user);
}

// ── Scene 6: Nether Portal (Detailed 70x17 Minecraft Obsidian Blocks + Vortex)

fn render_nether(g: &mut Grid, t: f32, si: &SysInfo, user: &str) {
    g.clear();
    // Classic chunky Minecraft obsidian portal frame (24 cols × 13 rows, zero bg boxes!)
    let x0: isize = 36;
    let x1: isize = 59;
    let y0: isize = 1;
    let y1: isize = 13;
    let cx = (x0 + x1) as f32 * 0.5;
    let cy = (y0 + y1) as f32 * 0.5;

    // Detailed cracked basalt & glowing magma-vein floor bathed in purple portal light (rows 13..16)
    for y in 13..ROWS {
        for x in 25..COLS {
            let d = ((x as f32 - cx) * 0.40).hypot((y as f32 - 13.0) * 1.35);
            let glow = (1.0 - d / 13.0).clamp(0.0, 1.0);
            let n = hash(x as f32 * 3.0, y as f32 * 7.0);
            // Subtle rare glowing magma cracks in the outer basalt floor!
            if n > 0.93 && glow < 0.55 {
                let pulse = 0.7 + 0.3 * (t * 3.5 + x as f32).sin();
                g.set(x as isize, y as isize, '~', 238.0 * pulse, 92.0 * pulse, 34.0 * pulse, 0.78);
            } else {
                let ch = if n > 0.65 {
                    '#'
                } else if n > 0.35 {
                    '='
                } else {
                    ':'
                };
                g.set(
                    x as isize,
                    y as isize,
                    ch,
                    lerp(44.0, 204.0, glow),
                    lerp(26.0, 84.0, glow),
                    lerp(38.0, 248.0, glow),
                    glow * 0.58,
                );
            }
        }
    }

    // Floating purple Nether portal sparks drifting outward
    for i in 0..24 {
        let fi = i as f32;
        let p = (t * (0.35 + hash(fi, 1.0) * 0.35) + hash(fi, 2.0)).fract();
        let ang = hash(fi, 3.0) * std::f32::consts::TAU;
        let px = cx + ang.cos() * (9.5 + p * 12.0);
        let py = cy + ang.sin() * (4.0 + p * 5.2) - p * 1.2;
        if px > 25.0 && py >= 0.0 && py < 13.5 {
            let fade = (p * std::f32::consts::PI).sin();
            let ch = if fade > 0.65 {
                '*'
            } else if fade > 0.35 {
                '°'
            } else {
                '·'
            };
            g.set(
                px as isize,
                py as isize,
                ch,
                228.0 * fade + 27.0,
                112.0 * fade + 22.0,
                255.0 * fade,
                fade * 0.92,
            );
        }
    }

    // Rich swirling Minecraft Nether Portal vortex interior
    let swirl_chars = b"~=+*%#@8";
    for y in (y0 + 2)..=(y1 - 1) {
        for x in (x0 + 3)..=(x1 - 3) {
            let dx = (x as f32 - cx) * 0.48;
            let dy = y as f32 - cy;
            let dist = dx.hypot(dy);
            let ang = dy.atan2(dx);
            let w1 = (dist * 1.45 - t * 4.2 + ang * 2.0).sin();
            let w2 = (x as f32 * 0.42 + y as f32 * 0.56 - t * 3.1).cos();
            let v = (0.50 + w1 * 0.30 + w2 * 0.22).clamp(0.10, 1.0);
            let ch = pick_char(swirl_chars, v);
            g.set(
                x,
                y,
                ch,
                lerp(132.0, 255.0, v),
                lerp(38.0, 172.0, v * v),
                255.0,
                v * 0.95,
            );
        }
    }

    // Weighty Minecraft Obsidian block frame (█/▓ textured blocks + glowing crying obsidian!)
    for y in y0..=y1 {
        for x in x0..=x1 {
            let is_top = y <= y0 + 1;
            let is_bot = y == y1;
            let is_side = x <= x0 + 2 || x >= x1 - 2;
            let is_corner = y == y0 && (x == x0 || x == x1);
            if is_corner || !(is_top || is_bot || is_side) {
                continue;
            }
            let n = hash(x as f32 * 5.0, y as f32 * 11.0);
            let is_crying = n > 0.87;
            let is_inner = x == x0 + 2 || x == x1 - 2 || y == y0 + 1 || y == y1;
            if is_crying {
                let pulse = 0.72 + 0.28 * (t * 4.0 + x as f32).sin();
                g.set(x, y, '▓', 218.0 * pulse, 118.0 * pulse, 255.0 * pulse, 0.95);
            } else {
                let ch = if x % 3 == 0 || y % 2 == 0 { '▓' } else { '█' };
                let r = if is_inner { 82.0 } else { 44.0 } + n * 18.0;
                let gr = if is_inner { 44.0 } else { 28.0 } + n * 12.0;
                let b = if is_inner { 132.0 } else { 76.0 } + n * 26.0;
                g.set(x, y, ch, r, gr, b, if is_inner { 0.35 } else { 0.0 });
            }
        }
    }

    // Flanking twin soul-fire / nether-fire braziers on the stone steps beside the portal!
    let f = 0.75 + 0.25 * (t * 7.2).sin();
    for &bx in &[x0 - 3, x1 + 3] {
        g.set(bx, y1 - 2, if f > 0.85 { '^' } else { '*' }, 255.0 * f, 152.0 * f, 58.0 * f, 0.92);
        g.set(bx, y1 - 1, '▓', 238.0 * f, 96.0 * f, 32.0 * f, 0.85);
        g.set(bx, y1, '▄', 82.0, 68.0, 96.0, 0.15);
    }

    draw_hud(g, "◈ NETHER PORTAL", (216.0, 128.0, 255.0), (136.0, 112.0, 168.0), si, user);
}

fn render_scene(scene: Scene, g: &mut Grid, t: f32, si: &SysInfo, user: &str) {
    match scene {
        Scene::Eclipse => render_eclipse(g, t, si, user),
        Scene::Bonfire => render_bonfire(g, t, si, user),
        Scene::Torii => render_torii(g, t, si, user),
        Scene::Portal => render_portal(g, t, si, user),
        Scene::Grimoire => render_grimoire(g, t, si, user),
        Scene::Nether => render_nether(g, t, si, user),
    }
}

// ── Terminal TTY & PID helpers ───────────────────────────────────────────────

fn is_stdout_tty() -> bool {
    unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 }
}

fn is_full_tty() -> bool {
    unsafe { libc::isatty(libc::STDIN_FILENO) == 1 && libc::isatty(libc::STDOUT_FILENO) == 1 }
}

fn term_pixel_size() -> (usize, usize, usize, usize) {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0
            && ws.ws_col > 0
            && ws.ws_row > 0
        {
            (
                ws.ws_col as usize,
                ws.ws_row as usize,
                ws.ws_xpixel as usize,
                ws.ws_ypixel as usize,
            )
        } else {
            (COLS, 26, 0, 0)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct HdLayout {
    banner_rows: usize,
    img_col: usize,
    img_top: usize,
    img_cols: usize,
    img_rows: usize,
    px_w: usize,
    px_h: usize,
}

fn compute_hd_layout(
    cols: usize,
    rows: usize,
    xpixel: usize,
    ypixel: usize,
    proto: GraphicsProto,
) -> Option<HdLayout> {
    if cols < 46 || rows < 10 {
        return None;
    }
    let banner_rows = HD_BANNER_ROWS.min(rows.saturating_sub(2));
    if banner_rows < 8 {
        return None;
    }
    let img_col = HUD_WIDTH + 1; // col 24 (1-indexed), tight clean gap right beside HUD
    let avail_cols = cols.saturating_sub(img_col - 1);
    if avail_cols < 16 {
        return None;
    }

    // Sanity-check cell aspect ratio to guard against transient TIOCGWINSZ races during tiling
    let (cell_w, cell_h) = if xpixel > 0 && ypixel > 0 && cols > 0 && rows > 0 {
        let cw = xpixel as f32 / cols as f32;
        let ch = ypixel as f32 / rows as f32;
        let ratio = ch / cw.max(1.0);
        if (4.0..=48.0).contains(&cw)
            && (8.0..=96.0).contains(&ch)
            && (1.25..=3.2).contains(&ratio)
        {
            (cw, ch)
        } else {
            (11.0, 24.0)
        }
    } else {
        (11.0, 24.0)
    };

    let max_px_w = avail_cols as f32 * cell_w;
    let max_px_h = banner_rows as f32 * cell_h;
    let mut scale = (max_px_w / gfx::IMG_W as f32).min(max_px_h / gfx::IMG_H as f32);
    // For Sixel, prefer exact 1:1 native 560x330 pixel sharpness whenever it fits (never stretch unevenly!)
    if proto == GraphicsProto::Sixel && scale > 1.0 {
        scale = 1.0;
    }

    let (px_w, px_h) = if proto == GraphicsProto::Sixel && (scale - 1.0).abs() < 0.001 {
        (gfx::IMG_W, gfx::IMG_H)
    } else {
        let pw = ((gfx::IMG_W as f32 * scale).round() as usize).clamp(120, max_px_w as usize);
        let ph = ((((gfx::IMG_H as f32 * scale).round() as usize).min(max_px_h as usize)) / 6) * 6;
        (pw, ph.max(60))
    };

    let img_cols = ((px_w as f32) / cell_w).ceil().max(12.0) as usize;
    let img_rows = ((px_h as f32) / cell_h).ceil().max(6.0) as usize;
    let img_cols = img_cols.min(avail_cols);
    let img_rows = img_rows.min(banner_rows);
    let img_top = 1 + banner_rows.saturating_sub(img_rows) / 2;

    Some(HdLayout {
        banner_rows,
        img_col,
        img_top,
        img_cols,
        img_rows,
        px_w,
        px_h,
    })
}

fn emit_hd_graphic(
    out: &mut String,
    proto: GraphicsProto,
    renderer: &mut BloomRenderer,
    layout: HdLayout,
    clear_sixel_cells: bool,
) {
    use std::fmt::Write as _;

    match proto {
        GraphicsProto::Kitty => {
            let _ = write!(out, "\x1b[{};{}H", layout.img_top, layout.img_col);
            renderer.emit_kitty_graphics(out, layout.img_cols, layout.img_rows);
        }
        GraphicsProto::Sixel => {
            if clear_sixel_cells {
                for row in 1..=layout.banner_rows {
                    let _ = write!(out, "\x1b[{};{}H\x1b[K", row, layout.img_col);
                }
            }
            let _ = write!(out, "\x1b[{};{}H", layout.img_top, layout.img_col);
            renderer.emit_sixel(out, layout.px_w, layout.px_h);
        }
        GraphicsProto::Text => {}
    }
}

fn push_spaces(out: &mut String, count: usize) {
    for _ in 0..count {
        out.push(' ');
    }
}

/// Encodes one fixed-width (HUD_WIDTH = 24 cols) native text HUD row without emitting `\x1b[K`,
/// so the right-hand Kitty Graphics placement at col 26+ is never disturbed.
fn encode_hd_hud_row(
    out: &mut String,
    scene: Scene,
    si: &SysInfo,
    user: &str,
    y: usize,
    banner_rows: usize,
) {
    use std::fmt::Write as _;

    let hud_top = banner_rows.saturating_sub(8) / 2;
    let (title, trgb, lrgb) = scene_hud_style(scene);

    if y < hud_top || y >= hud_top + 8 {
        out.push_str("\x1b[0m");
        push_spaces(out, HUD_WIDTH);
        return;
    }

    let rel = y - hud_top;
    match rel {
        0 => {
            let t = trunc(title, HUD_WIDTH - 2);
            let len = t.chars().count();
            let _ = write!(out, "\x1b[0m \x1b[1;38;2;{};{};{}m{}\x1b[0m", trgb.0, trgb.1, trgb.2, t);
            push_spaces(out, HUD_WIDTH.saturating_sub(1 + len));
        }
        1 => {
            let dr = (lrgb.0 as f32 * 0.62) as u8;
            let dg = (lrgb.1 as f32 * 0.62) as u8;
            let db = (lrgb.2 as f32 * 0.62) as u8;
            let _ = write!(
                out,
                "\x1b[0m \x1b[38;2;{};{};{}m─────────────────────\x1b[0m",
                dr, dg, db
            );
            push_spaces(out, HUD_WIDTH.saturating_sub(22));
        }
        2 => {
            out.push_str("\x1b[0m");
            push_spaces(out, HUD_WIDTH);
        }
        3..=7 => {
            let (k, raw_v): (&str, &str) = match rel {
                3 => ("user", user),
                4 => ("os  ", &si.os),
                5 => ("kern", &si.kernel),
                6 => ("ip  ", &si.local_ip),
                _ => ("pkgs", &si.packages),
            };
            let v = trunc(raw_v, 16);
            let vlen = v.chars().count();
            let _ = write!(
                out,
                "\x1b[0m \x1b[38;2;{};{};{}m{} \x1b[38;2;228;234;244m{}\x1b[0m",
                lrgb.0, lrgb.1, lrgb.2, k, v
            );
            push_spaces(out, HUD_WIDTH.saturating_sub(6 + vlen));
        }
        _ => {
            out.push_str("\x1b[0m");
            push_spaces(out, HUD_WIDTH);
        }
    }
}

fn encode_hd_hud_pinned(
    out: &mut String,
    scene: Scene,
    si: &SysInfo,
    user: &str,
    banner_rows: usize,
) {
    use std::fmt::Write as _;

    for y in 0..banner_rows {
        let _ = write!(out, "\x1b[{};1H", y + 1);
        encode_hd_hud_row(out, scene, si, user, y, banner_rows);
    }
}

pub fn dump_hd_rgba(scene: Scene, path: &str) -> io::Result<()> {
    let mut grid = HdGrid::new();
    let mut renderer = BloomRenderer::new();
    render_hd_scene(scene, &mut grid, 2.4);
    renderer.rasterize(&grid);
    std::fs::write(path, &renderer.rgba)
}

fn pid_file_for_tty() -> Option<String> {
    unsafe {
        let ptr = libc::ttyname(libc::STDOUT_FILENO);
        if ptr.is_null() {
            return None;
        }
        let cstr = std::ffi::CStr::from_ptr(ptr);
        let name = cstr.to_string_lossy().replace('/', "-");
        Some(format!("/tmp/greeting-tty-{}.pid", name))
    }
}

fn kill_existing_daemon(pid_path: &str) {
    if let Ok(contents) = std::fs::read_to_string(pid_path) {
        if let Some(pid_str) = contents.split_whitespace().next() {
            if let Ok(pid) = pid_str.parse::<libc::pid_t>() {
                if pid > 1 {
                    unsafe {
                        libc::kill(pid, libc::SIGTERM);
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_file(pid_path);
}

fn write_all_retry(stdout: &mut io::StdoutLock<'_>, mut data: &[u8]) -> io::Result<()> {
    while !data.is_empty() {
        match stdout.write(data) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::WriteZero, "eof")),
            Ok(n) => data = &data[n..],
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2));
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    stdout.flush()
}

struct TermGuard {
    orig_termios: libc::termios,
    active: bool,
}

impl TermGuard {
    fn enter() -> Option<Self> {
        unsafe {
            let mut orig_termios: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut orig_termios) != 0 {
                return None;
            }
            let mut raw = orig_termios;
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);
            raw.c_cc[libc::VMIN] = 0;
            raw.c_cc[libc::VTIME] = 0;
            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw) != 0 {
                return None;
            }
            libc::signal(
                libc::SIGINT,
                handle_stop_signal as *const () as libc::sighandler_t,
            );
            let _ = io::stdout().write_all(b"\x1b[?25l");
            let _ = io::stdout().flush();
            Some(Self {
                orig_termios,
                active: true,
            })
        }
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                let _ = libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.orig_termios);
                libc::signal(libc::SIGINT, libc::SIG_DFL);
            }
            let _ = io::stdout().write_all(b"\x1b[0m\x1b[?25h");
            let _ = io::stdout().flush();
            self.active = false;
        }
    }
}

enum KeyAction {
    None,
    Exit,
    Pick(Scene),
    NextRandom,
    Prev,
    ToggleLock,
}

fn poll_key() -> KeyAction {
    let mut pfd = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN,
        revents: 0,
    };
    let ready = unsafe { libc::poll(&mut pfd, 1, 0) };
    if ready <= 0 || (pfd.revents & libc::POLLIN) == 0 {
        return KeyAction::None;
    }

    let mut buf = [0u8; 8];
    let n = match io::stdin().read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return KeyAction::None,
    };

    if n >= 3 && buf[0] == 0x1b && buf[1] == b'[' {
        return match buf[2] {
            b'C' => KeyAction::NextRandom,
            b'D' => KeyAction::Prev,
            _ => KeyAction::None,
        };
    }

    match buf[0] {
        b'1' => KeyAction::Pick(Scene::Eclipse),
        b'2' => KeyAction::Pick(Scene::Bonfire),
        b'3' => KeyAction::Pick(Scene::Torii),
        b'4' => KeyAction::Pick(Scene::Portal),
        b'5' => KeyAction::Pick(Scene::Grimoire),
        b'6' => KeyAction::Pick(Scene::Nether),
        b' ' | b'n' | b'N' | b'r' | b'R' => KeyAction::NextRandom,
        b'p' | b'P' | b'l' | b'L' => KeyAction::ToggleLock,
        b'\r' | b'\n' | b'q' | b'Q' | 0x1b | 0x03 | 0x04 => KeyAction::Exit,
        _ => KeyAction::None,
    }
}

// ── Public entry points ──────────────────────────────────────────────────────

pub fn print_static(scene: Option<Scene>, force_text: bool, si: &SysInfo) {
    let mut rng = rand::thread_rng();
    let active = scene.unwrap_or_else(|| Scene::random(&mut rng));
    let t = rng.gen_range(1.2_f32..18.0_f32);
    let user = username();

    let (cols, rows, xpixel, ypixel) = term_pixel_size();
    let proto = if !force_text && is_stdout_tty() {
        gfx::detect_graphics_proto(None)
    } else {
        GraphicsProto::Text
    };
    if proto != GraphicsProto::Text {
        if let Some(layout) = compute_hd_layout(cols, rows, xpixel, ypixel, proto) {
            let mut hd_grid = HdGrid::new();
            let mut renderer = BloomRenderer::new();
            render_hd_scene(active, &mut hd_grid, t);
            renderer.rasterize(&hd_grid);

            let mut out = String::with_capacity(96_000);
            use std::fmt::Write as _;
            for y in 0..layout.banner_rows {
                encode_hd_hud_row(&mut out, active, si, &user, y, layout.banner_rows);
                out.push_str("\x1b[K\n");
            }
            out.push_str("\x1b7"); // save cursor below banner
            let up_rows = layout.banner_rows;
            let img_offset = layout.img_top.saturating_sub(1);
            let _ = write!(out, "\x1b[{}A\x1b[{}G", up_rows, layout.img_col);
            if img_offset > 0 {
                let _ = write!(out, "\x1b[{}B", img_offset);
            }
            match proto {
                GraphicsProto::Kitty => {
                    renderer.emit_kitty_graphics(&mut out, layout.img_cols, layout.img_rows);
                }
                GraphicsProto::Sixel => {
                    renderer.emit_sixel(&mut out, layout.px_w, layout.px_h);
                }
                GraphicsProto::Text => {}
            }
            out.push_str("\x1b8\r"); // restore cursor cleanly below banner
            let mut stdout = io::stdout().lock();
            let _ = write_all_retry(&mut stdout, out.as_bytes());
            return;
        }
    }

    let mut grid = Grid::new();
    render_scene(active, &mut grid, t, si, &user);

    let mut out = String::with_capacity(ROWS * COLS * 20);
    grid.encode_ansi(&mut out, cols, ROWS, true);
    let mut stdout = io::stdout().lock();
    let _ = write_all_retry(&mut stdout, out.as_bytes());
}

pub fn start_background_banner(
    initial: Option<Scene>,
    interval_secs: u64,
    force_text: bool,
    si: &SysInfo,
) {
    if !is_stdout_tty() {
        print_static(initial, force_text, si);
        return;
    }

    let (cols, rows, xpixel, ypixel) = term_pixel_size();
    let visible_rows = ROWS.min(rows.saturating_sub(2));
    if visible_rows < 6 {
        print_static(initial, force_text, si);
        return;
    }

    let mut rng = rand::thread_rng();
    let active = initial.unwrap_or_else(|| Scene::random(&mut rng));
    let user = username();

    if let Some(ref pid_path) = pid_file_for_tty() {
        kill_existing_daemon(pid_path);
    }

    let proto = if force_text {
        GraphicsProto::Text
    } else {
        gfx::detect_graphics_proto(None)
    };
    let hd_layout = if proto != GraphicsProto::Text {
        compute_hd_layout(cols, rows, xpixel, ypixel, proto)
    } else {
        None
    };

    if let Some(layout) = hd_layout {
        let mut hd_grid = HdGrid::new();
        let mut renderer = BloomRenderer::new();
        render_hd_scene(active, &mut hd_grid, 1.8);
        renderer.rasterize(&hd_grid);

        let mut out = String::with_capacity(96_000);
        out.push_str("\x1b[?2026h\x1b[?7l");
        if proto == GraphicsProto::Kitty {
            gfx::emit_kitty_delete(&mut out);
        }
        out.push_str("\x1b[2J");
        encode_hd_hud_pinned(&mut out, active, si, &user, layout.banner_rows);
        emit_hd_graphic(&mut out, proto, &mut renderer, layout, false);
        {
            use std::fmt::Write as _;
            let _ = write!(out, "\x1b[{};1H\x1b[?7h\x1b[?2026l", layout.banner_rows + 2);
        }
        {
            let mut stdout = io::stdout().lock();
            let _ = write_all_retry(&mut stdout, out.as_bytes());
        }
    } else {
        let mut grid = Grid::new();
        render_scene(active, &mut grid, 1.8, si, &user);

        let mut out = String::with_capacity(ROWS * COLS * 20 + 64);
        out.push_str("\x1b[?2026h\x1b[?7l\x1b[2J");
        grid.encode_ansi_pinned(&mut out, cols, visible_rows);
        {
            use std::fmt::Write as _;
            let _ = write!(out, "\x1b[{};1H\x1b[?7h\x1b[?2026l", visible_rows + 2);
        }
        {
            let mut stdout = io::stdout().lock();
            let _ = write_all_retry(&mut stdout, out.as_bytes());
        }
    }

    let shell_pid = unsafe { libc::getppid() };
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };

    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--daemon")
        .arg("--shell-pid")
        .arg(shell_pid.to_string())
        .arg("--scene")
        .arg((active.index() + 1).to_string())
        .arg("--interval")
        .arg(interval_secs.to_string());
    if force_text {
        cmd.arg("--text");
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::null());

    if let Ok(child) = cmd.spawn() {
        if let Some(pid_path) = pid_file_for_tty() {
            let _ = std::fs::write(&pid_path, format!("{} 0\n", child.id()));
        }
    }
}

pub fn run_daemon(
    initial: Option<Scene>,
    interval_secs: u64,
    shell_pid: i32,
    force_text: bool,
    si: &SysInfo,
) {
    if !is_stdout_tty() {
        return;
    }

    unsafe {
        libc::setpgid(0, 0);
        libc::signal(libc::SIGTTOU, libc::SIG_IGN);
        libc::signal(libc::SIGTTIN, libc::SIG_IGN);
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(
            libc::SIGTERM,
            handle_stop_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGHUP,
            handle_stop_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGWINCH,
            handle_winch_signal as *const () as libc::sighandler_t,
        );
    }

    STOP_REQUESTED.store(false, Ordering::SeqCst);
    RESIZED.store(false, Ordering::SeqCst);

    let pid_path = pid_file_for_tty();
    let my_pid = std::process::id();
    let shell_pgid = unsafe { libc::getpgid(shell_pid) };

    let mut rng = rand::thread_rng();
    let mut current = initial.unwrap_or_else(|| Scene::random(&mut rng));
    let user = username();
    let proto = if force_text {
        GraphicsProto::Text
    } else {
        gfx::detect_graphics_proto(Some(shell_pid))
    };
    let use_hd = proto != GraphicsProto::Text;

    let start = Instant::now();
    let mut last_switch = Instant::now();
    let mut seen_shell_fg = false;
    let mut last_size = term_pixel_size();
    let mut pending_resize_since: Option<Instant> = None;
    let mut last_hud_scene: Option<Scene> = Some(current);

    let mut grid = Grid::new();
    let mut hd_grid = if use_hd { Some(HdGrid::new()) } else { None };
    let mut renderer = if use_hd {
        Some(BloomRenderer::new())
    } else {
        None
    };
    let mut frame_buf = String::with_capacity(if use_hd { 96_000 } else { ROWS * COLS * 24 + 128 });

    thread::sleep(Duration::from_millis(70));

    loop {
        if STOP_REQUESTED.load(Ordering::SeqCst) {
            break;
        }

        if shell_pid > 1 && unsafe { libc::kill(shell_pid, 0) } != 0 {
            break;
        }

        let fg_pgrp = unsafe { libc::tcgetpgrp(libc::STDOUT_FILENO) };
        if fg_pgrp < 0 {
            break;
        }
        if shell_pgid > 0 {
            if fg_pgrp == shell_pgid {
                seen_shell_fg = true;
            } else if seen_shell_fg {
                break;
            } else if start.elapsed() > Duration::from_secs(2) {
                break;
            }
        }

        let size = term_pixel_size();
        if size != last_size || RESIZED.swap(false, Ordering::SeqCst) {
            last_size = size;
            pending_resize_since = Some(Instant::now());
            thread::sleep(Duration::from_millis(FRAME_MS));
            continue;
        }

        let (cols, rows, xpixel, ypixel) = size;
        let hd_layout = if use_hd {
            compute_hd_layout(cols, rows, xpixel, ypixel, proto)
        } else {
            None
        };

        if let Some(since) = pending_resize_since {
            if since.elapsed() < Duration::from_millis(80) {
                thread::sleep(Duration::from_millis(25));
                continue;
            }
            pending_resize_since = None;

            let t = start.elapsed().as_secs_f32() + 1.8;
            frame_buf.clear();
            frame_buf.push_str("\x1b[?2026h\x1b[?7l\x1b[?25l");
            if proto == GraphicsProto::Kitty {
                gfx::emit_kitty_delete(&mut frame_buf);
            }
            frame_buf.push_str("\x1b[2J");

            if let (Some(layout), Some(ref mut hg), Some(ref mut ren)) =
                (hd_layout, hd_grid.as_mut(), renderer.as_mut())
            {
                render_hd_scene(current, hg, t);
                ren.rasterize(hg);
                encode_hd_hud_pinned(&mut frame_buf, current, si, &user, layout.banner_rows);
                last_hud_scene = Some(current);
                emit_hd_graphic(&mut frame_buf, proto, ren, layout, false);
                {
                    use std::fmt::Write as _;
                    let _ = write!(frame_buf, "\x1b[{};1H", layout.banner_rows + 2);
                }
            } else {
                let visible_rows = ROWS.min(rows.saturating_sub(2));
                render_scene(current, &mut grid, t, si, &user);
                if visible_rows >= 4 && cols >= 20 {
                    grid.encode_ansi_pinned(&mut frame_buf, cols, visible_rows);
                    use std::fmt::Write as _;
                    let _ = write!(frame_buf, "\x1b[{};1H", visible_rows + 2);
                } else {
                    frame_buf.push_str("\x1b[1;1H");
                }
                last_hud_scene = None;
            }
            frame_buf.push_str("\x1b[?7h\x1b[?25h\x1b[?2026l");

            {
                let mut stdout = io::stdout().lock();
                if write_all_retry(&mut stdout, frame_buf.as_bytes()).is_err() {
                    break;
                }
            }
            if shell_pid > 1 {
                unsafe {
                    libc::kill(shell_pid, libc::SIGWINCH);
                }
            }
            thread::sleep(Duration::from_millis(60));
            continue;
        }

        if interval_secs > 0 && last_switch.elapsed() >= Duration::from_secs(interval_secs) {
            current = current.random_other(&mut rng);
            last_switch = Instant::now();
        }

        let t = start.elapsed().as_secs_f32() + 1.8;

        if let (Some(layout), Some(ref mut hg), Some(ref mut ren)) =
            (hd_layout, hd_grid.as_mut(), renderer.as_mut())
        {
            render_hd_scene(current, hg, t);
            ren.rasterize(hg);

            frame_buf.clear();
            frame_buf.push_str("\x1b[?2026h\x1b7\x1b[?7l\x1b[?25l");
            if last_hud_scene != Some(current) {
                encode_hd_hud_pinned(&mut frame_buf, current, si, &user, layout.banner_rows);
                last_hud_scene = Some(current);
            }
            emit_hd_graphic(&mut frame_buf, proto, ren, layout, true);
            frame_buf.push_str("\x1b[?7h\x1b8\x1b[?25h\x1b[?2026l");

            {
                let mut stdout = io::stdout().lock();
                if write_all_retry(&mut stdout, frame_buf.as_bytes()).is_err() {
                    break;
                }
            }

            thread::sleep(Duration::from_millis(HD_FRAME_MS));
        } else {
            let visible_rows = ROWS.min(rows.saturating_sub(2));
            if visible_rows < 4 || cols < 20 {
                thread::sleep(Duration::from_millis(FRAME_MS));
                continue;
            }

            render_scene(current, &mut grid, t, si, &user);

            frame_buf.clear();
            frame_buf.push_str("\x1b[?2026h\x1b7\x1b[?7l\x1b[?25l");
            grid.encode_ansi_pinned(&mut frame_buf, cols, visible_rows);
            frame_buf.push_str("\x1b[?7h\x1b8\x1b[?25h\x1b[?2026l");

            {
                let mut stdout = io::stdout().lock();
                if write_all_retry(&mut stdout, frame_buf.as_bytes()).is_err() {
                    break;
                }
            }

            thread::sleep(Duration::from_millis(FRAME_MS));
        }
    }

    if let Some(path) = pid_path {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if contents
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                == Some(my_pid)
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

pub fn run_foreground(
    initial: Option<Scene>,
    interval_secs: u64,
    force_text: bool,
    si: &SysInfo,
) {
    if !is_full_tty() {
        print_static(initial, force_text, si);
        return;
    }

    let _guard = match TermGuard::enter() {
        Some(g) => g,
        None => {
            print_static(initial, force_text, si);
            return;
        }
    };

    STOP_REQUESTED.store(false, Ordering::SeqCst);

    let mut rng = rand::thread_rng();
    let mut current = initial.unwrap_or_else(|| Scene::random(&mut rng));
    let mut locked = interval_secs == 0;
    let user = username();
    let proto = if force_text {
        GraphicsProto::Text
    } else {
        gfx::detect_graphics_proto(None)
    };
    let use_hd = proto != GraphicsProto::Text;

    let start = Instant::now();
    let mut last_switch = Instant::now();
    let mut last_size = (0usize, 0usize, 0usize, 0usize);
    let mut last_hud_scene: Option<Scene> = None;

    let mut grid = Grid::new();
    let mut hd_grid = if use_hd { Some(HdGrid::new()) } else { None };
    let mut renderer = if use_hd {
        Some(BloomRenderer::new())
    } else {
        None
    };
    let mut frame_buf = String::with_capacity(if use_hd { 96_000 } else { ROWS * COLS * 24 + 64 });

    loop {
        if STOP_REQUESTED.load(Ordering::SeqCst) {
            break;
        }

        match poll_key() {
            KeyAction::Exit => break,
            KeyAction::Pick(s) => {
                current = s;
                last_switch = Instant::now();
            }
            KeyAction::NextRandom => {
                current = current.random_other(&mut rng);
                last_switch = Instant::now();
            }
            KeyAction::Prev => {
                current = current.prev();
                last_switch = Instant::now();
            }
            KeyAction::ToggleLock => {
                locked = !locked;
                last_switch = Instant::now();
            }
            KeyAction::None => {}
        }

        if !locked
            && interval_secs > 0
            && last_switch.elapsed() >= Duration::from_secs(interval_secs)
        {
            current = current.random_other(&mut rng);
            last_switch = Instant::now();
        }

        let size = term_pixel_size();
        let (cols, rows, xpixel, ypixel) = size;
        let resized = size != last_size;
        if resized {
            last_size = size;
            last_hud_scene = None;
        }

        let hd_layout = if use_hd {
            compute_hd_layout(cols, rows, xpixel, ypixel, proto)
        } else {
            None
        };
        let t = start.elapsed().as_secs_f32() + 1.8;

        frame_buf.clear();
        frame_buf.push_str("\x1b[?2026h\x1b[?7l");
        if resized {
            if proto == GraphicsProto::Kitty {
                gfx::emit_kitty_delete(&mut frame_buf);
            }
            frame_buf.push_str("\x1b[2J");
        }

        let sleep_ms = if let (Some(layout), Some(ref mut hg), Some(ref mut ren)) =
            (hd_layout, hd_grid.as_mut(), renderer.as_mut())
        {
            render_hd_scene(current, hg, t);
            ren.rasterize(hg);
            if last_hud_scene != Some(current) {
                encode_hd_hud_pinned(&mut frame_buf, current, si, &user, layout.banner_rows);
                last_hud_scene = Some(current);
            }
            emit_hd_graphic(&mut frame_buf, proto, ren, layout, !resized);
            {
                use std::fmt::Write as _;
                let _ = write!(
                    frame_buf,
                    "\x1b[{};1H\x1b[?7h\x1b[?2026l",
                    layout.banner_rows + 1
                );
            }
            HD_FRAME_MS
        } else {
            let visible_rows = ROWS.min(rows.saturating_sub(1));
            render_scene(current, &mut grid, t, si, &user);
            grid.encode_ansi_pinned(&mut frame_buf, cols, visible_rows);
            {
                use std::fmt::Write as _;
                let _ = write!(frame_buf, "\x1b[{};1H\x1b[?7h\x1b[?2026l", visible_rows + 1);
            }
            FRAME_MS
        };

        {
            let mut stdout = io::stdout().lock();
            if write_all_retry(&mut stdout, frame_buf.as_bytes()).is_err() {
                break;
            }
        }

        thread::sleep(Duration::from_millis(sleep_ms));
    }
}
