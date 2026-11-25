use crate::gfx::{HdGrid, HD_COLS, HD_ROWS};
use crate::scenes::Scene;

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
    if v < 0.30 {
        let k = v / 0.30;
        (lerp(95.0, 215.0, k), lerp(20.0, 48.0, k), lerp(8.0, 14.0, k))
    } else if v < 0.64 {
        let k = (v - 0.30) / 0.34;
        (lerp(215.0, 255.0, k), lerp(48.0, 162.0, k), lerp(14.0, 38.0, k))
    } else {
        let k = (v - 0.64) / 0.36;
        (255.0, lerp(162.0, 250.0, k), lerp(38.0, 215.0, k))
    }
}

fn pick_char(ramp: &[u8], v: f32) -> char {
    let idx = (v.clamp(0.0, 0.9999) * ramp.len() as f32) as usize;
    ramp[idx] as char
}

// ── Scene 1: Eclipse Knight (High-Res 80x30 Soulsborne Dark Knight + Bloom) ─
// Anchored at x=36, y=10 (30 cols wide, 20 rows tall)
const HD_KNIGHT_BODY: &[&str] = &[
    "         PPPCCCCCCPPP         ", // 0  high gorget
    "     PPPPPPCCCCCCCCPPPPPP     ", // 1  wide flared spiked pauldrons
    "   PPPPPPSCCCRRRRCCCCSPPPPP   ", // 2  layered shoulder plates + chest crest
    "   PPAAAASCCCRRRRCCCCSMMMMP   ", // 3  armored arms + sculpted breastplate
    "   PPAAAASCCCPPRRCCCCSMMMMP   ", // 4  tapered chest + midnight cape
    "    PAAA.SCCCCCCCCCCCS.MMMM   ", // 5  armored forearm reaching hilt
    "    AAAA.SCCCCCCCCCCCS.MMMM   ", // 6  waist plate
    "    AAG..GGGGGGGGGGGGG.MMMM   ", // 7  forged gold belt + double gem buckle
    "        .TTTTTTGGTTTTT.MMMM   ", // 8  tassets + royal indigo tabard
    "       ..TTTTTTGGTTTTTT.MMM   ", // 9  pleated skirt + gold brocade stripe
    "       ..TTTTTTGGTTTTTT.MMM   ", // 10
    "       ..TTTTTTGGTTTTTT..MM   ", // 11
    "      ...TTTTTTGGTTTTTT..MM   ", // 12
    "      ...TTTTT.GG.TTTTT..MM   ", // 13 split hem revealing armored legs
    "       ..KKKK..GG..KKKK..M    ", // 14 sculpted knee poleyns
    "       ..BBBB..GG..BBBB..M    ", // 15 articulated steel greaves
    "       ..BBBB......BBBB..M    ", // 16
    "       ..BBBB......BBBB.      ", // 17 ankle plates
    "      ..BBBBB......BBBBB..    ", // 18 tapered armored sabatons
    "     ..BBBBBB......BBBBBB..   ", // 19 grounded on ember ash
];

fn render_hd_eclipse(g: &mut HdGrid, t: f32) {
    g.clear();
    let bx0: isize = 33;
    let by0: isize = 10;
    let cx: f32 = bx0 as f32 + 14.5; // 47.5
    let head_y: f32 = 5.4;

    // 1. Ember-dusted ash floor (rows 28..29)
    for y in 28..HD_ROWS {
        for x in 2..HD_COLS as isize - 2 {
            let fx = x as f32;
            let d = ((fx - cx) * 0.032).abs();
            let n = hash(fx * 3.1, y as f32 * 7.3);
            if d < 1.6 && n > 0.48 {
                let fade = (1.0 - d / 1.6).clamp(0.0, 1.0);
                let ch = if n > 0.80 { '~' } else if n > 0.62 { '-' } else { '.' };
                g.set(x, y as isize, ch, 82.0 * fade, 56.0 * fade, 44.0 * fade, 0.08 * fade);
            }
        }
    }

    // 2. Ambient glowing embers drifting upward through dark void
    for i in 0..42 {
        let fi = i as f32;
        let sx = (hash(fi, 1.3) * HD_COLS as f32 + (t * 0.9 + fi).sin() * 1.6).rem_euclid(HD_COLS as f32);
        let sy = (hash(fi, 4.7) * HD_ROWS as f32 - t * (1.0 + hash(fi, 9.1) * 1.5)).rem_euclid(HD_ROWS as f32);
        let flicker = 0.45 + 0.55 * (t * 4.8 + fi * 2.3).sin();
        let ch = if flicker > 0.82 {
            '*'
        } else if flicker > 0.55 {
            '·'
        } else {
            '.'
        };
        g.set(
            sx as isize,
            sy as isize,
            ch,
            255.0 * flicker,
            138.0 * flicker,
            45.0 * flicker,
            flicker * 0.95,
        );
    }

    // 3. Roaring Diagonal Flaming Greatsword (from gauntlet (35, 17) down-left to tip (2, 25.6))
    let guard_x = 35.0_f32;
    let guard_y = 17.0_f32;
    let tip_x = 2.0_f32;
    let tip_y = 25.6_f32;
    let dx_s = tip_x - guard_x;
    let dy_s = tip_y - guard_y;
    let vx = dx_s;
    let vy = dy_s * 1.75;
    let l2 = vx * vx + vy * vy;

    // Volumetric multi-layered flames rising cleanly from the blade spine
    for y in 10..HD_ROWS {
        for x in 0..39 {
            let fx = x as f32;
            let fy = y as f32;
            let px = fx - guard_x;
            let py = (fy - guard_y) * 1.75;
            let proj = ((px * vx + py * vy) / l2).clamp(0.0, 1.0);
            let bx = guard_x + dx_s * proj;
            let by = guard_y + dy_s * proj;
            let dist_y = fy - by;
            let dist_x = (fx - bx).abs();

            if dist_y < 0.85 && dist_y > -5.5 && dist_x < 2.6 && proj > 0.03 && proj < 0.97 {
                let rise = (1.0 - (-dist_y).max(0.0) / 5.4).clamp(0.0, 1.0);
                let turb = fbm(fx * 0.28 - t * 2.8, fy * 0.38 + t * 5.2);
                let intensity = rise * turb * (1.28 - proj * 0.30);
                if intensity > 0.20 {
                    let v = ((intensity - 0.20) / 0.72).clamp(0.0, 1.0);
                    let (r, gr, b) = fire_color(v);
                    let ch = pick_char(b".,'~^*+&%#8@", v);
                    g.set(x as isize, y as isize, ch, r, gr, b, v * 1.15);
                }
            }
        }
    }

    // Incandescent molten-steel tapered greatsword blade
    for step in 2..=68 {
        let p = step as f32 / 68.0;
        let bx = guard_x + dx_s * p;
        let by = guard_y + dy_s * p;
        let ix = bx.round() as isize;
        let iy = by.round() as isize;
        let heat = (0.78 + 0.22 * (t * 6.5 - p * 9.0).sin() - p * 0.12).clamp(0.45, 1.0);
        let (fr, fg, fb) = fire_color(heat);
        let ch = if p > 0.95 {
            '<'
        } else if p > 0.80 {
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
            lerp(245.0, fr, 0.48),
            lerp(230.0, fg, 0.48),
            lerp(205.0, fb, 0.48),
            1.2,
        );
    }

    // Sculpted crossguard, leather grip & forged gold pommel
    let hilt: &[(isize, isize, char, (f32, f32, f32), f32)] = &[
        (34, 15, '\\', (236.0, 206.0, 140.0), 0.75),
        (35, 16, '\\', (244.0, 214.0, 148.0), 0.85),
        (35, 17, 'X', (255.0, 232.0, 168.0), 1.05),
        (35, 18, '/', (238.0, 206.0, 138.0), 0.85),
        (34, 19, '/', (218.0, 184.0, 118.0), 0.65),
        (36, 17, '=', (186.0, 148.0, 102.0), 0.25),
        (37, 17, '=', (186.0, 148.0, 102.0), 0.25),
        (39, 17, '◈', (248.0, 208.0, 124.0), 0.90),
    ];
    for &(hx, hy, ch, (hr, hg, hb), glow) in hilt {
        g.set(hx, hy, ch, hr, hg, hb, glow);
    }

    // 4. High-Detail Sculpted Armored Knight Body
    let steel_ramp = b".:-=+*#%&$8@";
    let cloth_ramp = b".:;=+%#&8";
    for (ry, row_str) in HD_KNIGHT_BODY.iter().enumerate() {
        let y = by0 + ry as isize;
        let fy = y as f32;
        for (rx, mask) in row_str.chars().enumerate() {
            if mask == ' ' || mask == '.' {
                continue;
            }
            let x = bx0 + rx as isize;
            let fx = x as f32;
            let dx = fx - cx;
            let nx = (dx / 13.0).clamp(-1.0, 1.0);
            let nz = (1.0 - nx * nx * 0.72).max(0.12).sqrt();
            let tex = hash(fx * 3.7, fy * 7.1);

            let top_glow = (1.0 - (fy - 9.0) / 8.5).clamp(0.0, 1.0) * (0.45 + 0.55 * nz);
            let sword_dist = ((fx - 34.0) * 0.085).hypot((fy - 18.0) * 0.14);
            let sword_light = (1.0 - sword_dist).clamp(0.0, 1.0)
                * (-nx * 0.85 + 0.45).clamp(0.0, 1.0)
                * (0.86 + 0.14 * (t * 7.0 + fy).sin());

            match mask {
                'P' => {
                    let edge = (dx.abs() - 6.8).clamp(0.0, 5.5) / 5.5;
                    let lum = (0.36 + nz * 0.42 + edge * 0.26 + top_glow * 0.65).clamp(0.18, 1.0);
                    let warm = (top_glow * 0.92 + sword_light * 0.82).clamp(0.0, 1.0);
                    let r = lerp(lerp(92.0, 228.0, lum), 255.0, warm * 0.72);
                    let gr = lerp(lerp(100.0, 234.0, lum), 186.0, warm * 0.70);
                    let b = lerp(lerp(122.0, 248.0, lum), 86.0, warm * 0.65);
                    let ch = if edge > 0.76 {
                        if dx < 0.0 { '/' } else { '\\' }
                    } else if ry == 1 && edge > 0.35 {
                        '='
                    } else {
                        pick_char(steel_ramp, lum * 0.85 + tex * 0.18)
                    };
                    g.set(x, y, ch, r, gr, b, warm * 0.72 + edge * 0.22);
                }
                'R' => {
                    let lum = (0.62 + top_glow * 0.48 + sword_light * 0.35).clamp(0.32, 1.0);
                    let warm = (top_glow * 0.78 + sword_light * 0.62).clamp(0.0, 1.0);
                    let r = lerp(202.0 * lum, 255.0, warm * 0.68);
                    let gr = lerp(212.0 * lum, 200.0, warm * 0.65);
                    let b = lerp(234.0 * lum, 112.0, warm * 0.62);
                    let ch = if dx.abs() < 0.7 { '|' } else { pick_char(b"=#%8@", lum) };
                    g.set(x, y, ch, r, gr, b, 0.62 + warm * 0.35);
                }
                'C' | 'A' => {
                    let spec = (nz * 0.85 - nx * 0.35).clamp(0.0, 1.0).powi(3);
                    let lum = (0.20 + nz * 0.38 + spec * 0.38 + top_glow * 0.52 + sword_light * 0.52)
                        .clamp(0.12, 1.0);
                    let warm = (top_glow * 0.78 + sword_light * 0.92).clamp(0.0, 1.0);
                    let r = lerp(lerp(62.0, 198.0, lum), 255.0, warm * 0.74);
                    let gr = lerp(lerp(70.0, 206.0, lum), 174.0, warm * 0.70);
                    let b = lerp(lerp(92.0, 226.0, lum), 82.0, warm * 0.66);
                    let ch = pick_char(steel_ramp, lum * 0.82 + tex * 0.22);
                    g.set(x, y, ch, r, gr, b, warm * 0.52);
                }
                'S' => {
                    let ch = if tex > 0.5 { ':' } else { ';' };
                    g.set(
                        x,
                        y,
                        ch,
                        54.0 + sword_light * 90.0,
                        60.0 + sword_light * 52.0,
                        82.0 + sword_light * 24.0,
                        0.08,
                    );
                }
                'M' => {
                    let sway = ((t * 2.4 - fy * 0.35).sin() * 0.55).round() as isize;
                    let shade = (0.26 + tex * 0.28 + top_glow * 0.25).clamp(0.14, 0.74);
                    let ch = if ry >= 14 {
                        if tex > 0.5 { '/' } else { ';' }
                    } else {
                        pick_char(b":;/|\\", shade)
                    };
                    g.set(
                        x + if ry >= 11 { sway } else { 0 },
                        y,
                        ch,
                        52.0 * shade + 24.0,
                        56.0 * shade + 28.0,
                        96.0 * shade + 44.0,
                        0.06,
                    );
                }
                'G' => {
                    let shade = (0.50 + nz * 0.45 + sword_light * 0.4).clamp(0.28, 1.0);
                    let ch = if ry == 7 {
                        if dx.abs() < 1.3 { '◈' } else { '=' }
                    } else if tex > 0.5 {
                        '&'
                    } else {
                        '%'
                    };
                    g.set(
                        x,
                        y,
                        ch,
                        lerp(132.0, 255.0, shade),
                        lerp(96.0, 212.0, shade),
                        lerp(32.0, 92.0, shade),
                        0.55 + sword_light * 0.35,
                    );
                }
                'T' => {
                    let fold = 0.72 + 0.28 * (dx * 1.55 + fy * 0.22).sin();
                    let shade = ((0.32 + nz * 0.55) * fold + sword_light * 0.36).clamp(0.16, 1.0);
                    let r = lerp(34.0, 82.0, shade) + sword_light * 102.0;
                    let gr = lerp(42.0, 98.0, shade) + sword_light * 46.0;
                    let b = lerp(96.0, 214.0, shade) + sword_light * 14.0;
                    let ch = pick_char(cloth_ramp, shade * 0.82 + tex * 0.2);
                    g.set(x, y, ch, r, gr, b, 0.14 + sword_light * 0.25);
                }
                'K' => {
                    let ch = match rx {
                        9 | 19 => '<',
                        12 | 22 => '>',
                        _ => '#',
                    };
                    let warm = (sword_light * 0.72).clamp(0.0, 1.0);
                    g.set(
                        x,
                        y,
                        ch,
                        lerp(184.0, 252.0, warm * 0.65),
                        lerp(194.0, 182.0, warm * 0.60),
                        lerp(222.0, 98.0, warm * 0.55),
                        0.48,
                    );
                }
                'B' => {
                    let local_cx = if dx < 0.0 { cx - 4.2 } else { cx + 4.2 };
                    let lnx = ((fx - local_cx) / 2.6).clamp(-1.0, 1.0);
                    let lnz = (1.0 - lnx * lnx).max(0.15).sqrt();
                    let lum = (0.30 + lnz * 0.52 + sword_light * 0.42).clamp(0.16, 1.0);
                    let warm = (sword_light * 0.72).clamp(0.0, 1.0);
                    let r = lerp(lerp(76.0, 208.0, lum), 250.0, warm * 0.65);
                    let gr = lerp(lerp(84.0, 218.0, lum), 168.0, warm * 0.60);
                    let b = lerp(lerp(106.0, 238.0, lum), 82.0, warm * 0.55);
                    let ch = if ry == 19 {
                        if (dx < 0.0 && rx == 7) || (dx > 0.0 && rx == 19) {
                            '/'
                        } else if (dx < 0.0 && rx == 12) || (dx > 0.0 && rx == 24) {
                            '\\'
                        } else {
                            '='
                        }
                    } else if lnz > 0.85 {
                        '#'
                    } else {
                        pick_char(b":=+%#8", lum)
                    };
                    g.set(x, y, ch, r, gr, b, 0.22 + warm * 0.35);
                }
                _ => {}
            }
        }
    }

    // 5. Blazing Solar-Eclipse Halo Head (dense swirling corona + intense Gaussian bloom!)
    for y in 0..=11 {
        for x in (cx as isize - 14)..=(cx as isize + 14) {
            let fx = x as f32;
            let fy = y as f32;
            let dx = (fx - cx) * 0.60;
            let dy = fy - head_y;
            let r = dx.hypot(dy);
            let ang = dy.atan2(dx);
            let wave = (ang * 5.0 + t * 3.1).sin() * 0.36 + (ang * 9.0 - t * 4.5).cos() * 0.22;
            let top_lift = if dy < 0.0 {
                -dy * 0.32 * fbm(fx * 0.32, fy * 0.38 + t * 3.6)
            } else {
                0.0
            };
            let ring_dist = (r - (3.55 + wave * 0.72)).abs();

            if r < 2.55 {
                g.set(x, y as isize, ' ', 0.0, 0.0, 0.0, 0.0);
                if r > 2.05 && hash(fx, fy) > 0.68 {
                    g.set(x, y as isize, ':', 125.0, 42.0, 14.0, 0.35);
                }
            } else if ring_dist < 1.65 + top_lift {
                let core = (1.0 - ring_dist / (1.65 + top_lift)).clamp(0.0, 1.0);
                let noise = fbm(fx * 0.34 + t.cos() * 0.5, fy * 0.42 + t * 4.0);
                let v = (core * (0.74 + noise * 0.58)).clamp(0.0, 1.0);
                if v > 0.14 {
                    let (cr, cg, cb) = fire_color(v);
                    let ch = pick_char(b".'~^*+%#8@", v);
                    g.set(x, y as isize, ch, cr, cg, cb, v * 1.25);
                }
            }
        }
    }
}

// ── Scene 2: Ruined Bonfire (High-Res 80x30 Gothic Sanctuary + Fire Bloom) ──

fn render_hd_bonfire(g: &mut HdGrid, t: f32) {
    g.clear();
    let fx = 33.0_f32;
    let fy = 22.0_f32;
    let pulse = 0.88 + 0.12 * (t * 7.5).sin() + 0.06 * (t * 13.1).cos();

    // Starry night sky
    for y in 0..14 {
        for x in 1..HD_COLS - 1 {
            if hash(x as f32 * 7.0, y as f32 * 13.0) > 0.988 {
                let tw = 0.5 + 0.5 * (t * 2.2 + x as f32).sin();
                g.set(
                    x as isize,
                    y as isize,
                    if tw > 0.72 { '*' } else { '·' },
                    155.0 * tw,
                    178.0 * tw,
                    225.0 * tw,
                    tw * 0.6,
                );
            }
        }
    }

    // Soaring ruined gothic sanctuary columns & crumbling pointed arch
    let stone_glyphs = b":;=#%H8";
    for y in 2..=23 {
        for x in 3..=62 {
            let x_f = x as f32;
            let y_f = y as f32;
            let is_left = (5..=11).contains(&x) && y >= 7;
            let is_right = (53..=59).contains(&x) && y >= 4;
            let is_mid_right = (45..=49).contains(&x) && y >= 6;
            let arch_dist = (((x_f - 22.0) * 0.65).hypot(y_f - 12.0) - 10.5).abs();
            let is_arch = arch_dist < 1.25 && y_f < 12.0 && (5..=31).contains(&x);

            if is_left || is_right || is_mid_right || is_arch {
                let d_fire = ((x_f - fx) * 0.45).hypot((y_f - fy) * 0.85);
                let warm = (1.0 - d_fire / 20.0).clamp(0.0, 1.0) * pulse;
                let n = hash(x_f * 3.0, y_f * 5.0);
                let ch = pick_char(stone_glyphs, 0.34 + warm * 0.52 + n * 0.2);
                g.set(
                    x as isize,
                    y as isize,
                    ch,
                    lerp(42.0, 232.0, warm),
                    lerp(50.0, 128.0, warm),
                    lerp(74.0, 56.0, warm),
                    warm * 0.55,
                );
            }
        }
    }

    // Cobblestone floor bathed in live radial warm firelight (rows 23..29)
    let floor_glyphs = b" .,:;-=~#%";
    for y in 23..HD_ROWS {
        for x in 2..HD_COLS - 2 {
            let x_f = x as f32;
            let y_f = y as f32;
            let d_fire = ((x_f - fx) * 0.42).hypot((y_f - fy) * 1.35);
            let light = (1.0 - d_fire / 24.0).clamp(0.0, 1.0) * pulse;
            let n = hash(x_f * 5.0, y_f * 11.0);
            if light > 0.06 || n > 0.72 {
                let ch = pick_char(floor_glyphs, light * 0.75 + n * 0.28);
                g.set(
                    x as isize,
                    y as isize,
                    ch,
                    lerp(34.0, 240.0, light),
                    lerp(38.0, 132.0, light),
                    lerp(54.0, 52.0, light),
                    light * 0.45,
                );
            }
        }
    }

    // Ash & glowing ember mound
    for y in 21..=25 {
        let w = (y - 20) as f32 * 4.4;
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
                lerp(82.0, 248.0, hot),
                lerp(46.0, 122.0, hot),
                lerp(36.0, 44.0, hot),
                hot * 0.68,
            );
        }
    }

    // Roaring multi-layered volumetric bonfire flame (rows 6..22)
    let flame_glyphs = b".'~^*+S&%#8@";
    for y in 6..=22 {
        let h = (22 - y) as f32 / 16.0;
        let sway = (t * 4.2 - h * 5.2).sin() * (h * 3.0);
        let radius = (1.0 - h * 0.80) * 8.4 + 0.8;
        for x in (fx as isize - 12)..=(fx as isize + 12) {
            let dx = (x as f32 - (fx + sway)).abs();
            if dx > radius + 1.8 {
                continue;
            }
            let edge = (1.0 - dx / radius).clamp(0.0, 1.0);
            let turb = fbm(x as f32 * 0.30, y as f32 * 0.34 + t * 5.2);
            let v = ((edge * 0.96 + turb * 0.44 - h * 0.42) * pulse).clamp(0.0, 1.0);
            if v > 0.16 {
                let (r, gr, b) = fire_color(((v - 0.16) / 0.78).clamp(0.0, 1.0));
                let ch = pick_char(flame_glyphs, v);
                g.set(x, y as isize, ch, r, gr, b, v * 1.3);
            }
        }
    }

    // Iconic Coiled Greatsword thrust diagonally into the bonfire heart
    let fxi = fx as isize;
    let sword = [
        (fxi + 4, 8, '◈'),
        (fxi + 4, 9, '║'),
        (fxi + 4, 10, '║'),
        (fxi + 2, 11, '═'),
        (fxi + 3, 11, '═'),
        (fxi + 4, 11, '╬'),
        (fxi + 5, 11, '═'),
        (fxi + 6, 11, '═'),
        (fxi + 3, 12, '/'),
        (fxi + 3, 13, 'S'),
        (fxi + 2, 14, '/'),
        (fxi + 2, 15, 'S'),
        (fxi + 1, 16, '/'),
        (fxi + 1, 17, 'S'),
        (fxi, 18, '/'),
        (fxi, 19, 'S'),
        (fxi - 1, 20, '/'),
        (fxi - 1, 21, '│'),
        (fxi - 1, 22, '│'),
    ];
    for (sx, sy, sch) in sword {
        let heat = ((sy as f32 - 10.0) / 12.0).clamp(0.0, 1.0);
        g.set(
            sx,
            sy,
            sch,
            lerp(202.0, 255.0, heat),
            lerp(212.0, 232.0, heat),
            lerp(228.0, 142.0, heat),
            0.95,
        );
    }

    // Rising glowing embers floating up into the night
    for i in 0..34 {
        let fi = i as f32;
        let p = (t * (0.28 + hash(fi, 1.0) * 0.28) + hash(fi, 2.0)).fract();
        let ex = fx + (hash(fi, 3.0) - 0.5) * 28.0 + (t * 2.6 + fi).sin() * 2.8;
        let ey = 22.0 - p * 21.0;
        let glow = (1.0 - p) * (0.6 + 0.4 * (t * 8.0 + fi).sin());
        g.set(
            ex as isize,
            ey as isize,
            if glow > 0.62 { '*' } else { '·' },
            255.0 * glow,
            152.0 * glow,
            48.0 * glow,
            glow * 1.15,
        );
    }
}

// ── Scene 3: Moonlit Torii (High-Res 80x30 Blood Moon + Crisp Block Shrine) ─
// 43 cols wide, exact center at col 21 (anchored x=19, y=8):
const HD_TORII: &[&str] = &[
    "  ▀▄▄                                 ▄▄▀  ", // 0  upturned kasagi roof tips
    "    ▀▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▀    ", // 1  curved upper kasagi beam
    "      ███████████████████████████████      ", // 2  heavy roof body
    "       ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀       ", // 3  underbeam shadow
    "          ███       ▓◈▓       ███          ", // 4  pillars + gilded central plaque
    "          ███       ▓▓▓       ███          ", // 5
    "      ▄▄███████████████████████████▄▄      ", // 6  wide nuki crossbeam
    "       ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀       ", // 7
    "          ███                 ███          ", // 8  shrine doorway opening
    "          ███                 ███          ", // 9
    "          ███                 ███          ", // 10
    "          ███                 ███          ", // 11
    "          ███                 ███          ", // 12
    "          ███                 ███          ", // 13
    "         ▄███▄               ▄███▄         ", // 14 stone water pedestals
];

fn render_hd_torii(g: &mut HdGrid, t: f32) {
    g.clear();
    let tx0: isize = 11;
    let ty0: isize = 8;
    let mcx = 32.0_f32; // exact center (11 + 21)
    let mcy = 11.2_f32;

    // Night stars
    for y in 0..22 {
        for x in 1..HD_COLS - 1 {
            if hash(x as f32 * 7.0, y as f32 * 13.0) > 0.986 {
                let tw = 0.5 + 0.5 * (t * 2.2 + x as f32).sin();
                g.set(
                    x as isize,
                    y as isize,
                    if tw > 0.72 { '*' } else { '·' },
                    188.0 * tw,
                    202.0 * tw,
                    238.0 * tw,
                    tw * 0.65,
                );
            }
        }
    }

    // Massive luminous Crimson-Silver Blood Moon with crater shading & lush optical bloom!
    let moon_ramp = b".:-=+*#%8@";
    for y in 0..23 {
        for x in 6..59 {
            let dx = (x as f32 - mcx) * 0.56;
            let dy = y as f32 - mcy;
            let d = dx.hypot(dy);
            if d < 9.8 {
                let core = (1.0 - d / 9.8).clamp(0.0, 1.0);
                let crater = fbm(x as f32 * 0.22, y as f32 * 0.28);
                let v = (0.30 + core * 0.62 + crater * 0.24).clamp(0.20, 1.0);
                let ch = pick_char(moon_ramp, v);
                g.set(
                    x as isize,
                    y as isize,
                    ch,
                    lerp(205.0, 255.0, core),
                    lerp(56.0, 208.0, core * core),
                    lerp(72.0, 188.0, core * core),
                    0.28 + core * 0.48,
                );
            } else if d < 12.2 && hash(x as f32, y as f32) > 0.68 {
                let fade = (1.0 - (d - 9.8) / 2.4).clamp(0.0, 1.0);
                g.set(
                    x as isize,
                    y as isize,
                    '·',
                    168.0 * fade,
                    48.0 * fade,
                    62.0 * fade,
                    fade * 0.5,
                );
            }
        }
    }

    // Distant pine-crested rocky islets flanking the shrine horizon (rows 19..22)
    let pines: &[(isize, isize)] = &[(4, 20), (8, 21), (57, 21), (61, 20)];
    for &(px, py) in pines {
        g.set(px, py - 2, '^', 56.0, 72.0, 96.0, 0.05);
        g.put_text(px - 1, py - 1, "/#\\", 48.0, 62.0, 86.0, 0.05);
        g.put_text(px - 2, py, "/#%#\\", 38.0, 52.0, 74.0, 0.05);
    }

    // Shimmering dark reflective lake with glowing crimson moon & pillar reflection (rows 23..29)
    for y in 23..HD_ROWS {
        let depth = (y - 23) as f32 / 6.0;
        for x in 1..HD_COLS - 1 {
            let fx = x as f32;
            let fy = y as f32;
            let wave = (fx * 0.34 + t * 2.8 + fy * 1.4).sin() * 0.5 + 0.5;
            let d_moon = (fx - mcx + (t * 2.1 + fy * 1.3).sin() * 1.6).abs();
            let moon_ref = (1.0 - d_moon / (13.0 - depth * 3.5)).clamp(0.0, 1.0);
            let pillar_ref = (fx - 22.0).abs() < 1.8 || (fx - 42.0).abs() < 1.8;
            if moon_ref > 0.10 {
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
                    lerp(142.0, 255.0, w),
                    lerp(42.0, 158.0, w),
                    lerp(50.0, 144.0, w),
                    moon_ref * 0.95,
                );
            } else if pillar_ref && hash(fx, fy) > 0.25 {
                g.set(x as isize, y as isize, '~', 128.0, 34.0, 44.0, 0.35);
            } else if hash(fx * 3.0, fy * 5.0) > 0.56 {
                g.set(
                    x as isize,
                    y as isize,
                    if wave > 0.5 { '~' } else { '-' },
                    46.0 + wave * 30.0,
                    62.0 + wave * 38.0,
                    96.0 + wave * 48.0,
                    0.08,
                );
            }
        }
    }

    // Drifting glowing sakura petals crossing the wind
    for i in 0..26 {
        let fi = i as f32;
        let px = ((hash(fi, 1.7) * HD_COLS as f32 + t * (3.2 + hash(fi, 2.3) * 2.4)) % HD_COLS as f32).abs();
        let py = (hash(fi, 4.1) * HD_ROWS as f32 + t * (1.1 + hash(fi, 5.9) * 0.8) + (t * 2.2 + fi).sin() * 0.7)
            .rem_euclid(HD_ROWS as f32);
        let ch = match i % 3 {
            0 => '*',
            1 => ',',
            _ => '·',
        };
        g.set(px as isize, py as isize, ch, 255.0, 192.0, 210.0, 0.95);
    }

    // Crisp lacquered Torii gate silhouette with vermilion rim & gilded central tablet!
    for (ry, line) in HD_TORII.iter().enumerate() {
        for (rx, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = tx0 + rx as isize;
            let y = ty0 + ry as isize;
            if ch == '◈' {
                g.set(x, y, '◈', 255.0, 224.0, 132.0, 0.95);
            } else if ch == '▓' {
                g.set(x, y, '▓', 188.0, 132.0, 54.0, 0.24);
            } else if ry == 14 {
                g.set(x, y, ch, 92.0, 82.0, 98.0, 0.10);
            } else {
                let is_top_rim = ry <= 1 || ry == 6 || rx == 10 || rx == 32;
                let is_deep = ry == 3 || ry == 7 || rx == 11 || rx == 31;
                if is_top_rim {
                    g.set(x, y, ch, 238.0, 58.0, 54.0, 0.42);
                } else if is_deep {
                    g.set(x, y, ch, 128.0, 22.0, 30.0, 0.08);
                } else {
                    g.set(x, y, ch, 184.0, 34.0, 40.0, 0.18);
                }
            }
        }
    }
}

// ── Scene 4: Astral Gate (High-Res 80x30 Carved Archway + Electric Vortex) ──

fn render_hd_portal(g: &mut HdGrid, t: f32) {
    g.clear();
    let cx = 33.0_f32;
    let cy = 14.5_f32;

    // Hypnotic multi-arm violet-to-cyan-to-white swirling void vortex
    let ramp = b" .:-=+*#%8@";
    for y in 4..26 {
        for x in 13..54 {
            let dx = (x as f32 - cx) * 0.56;
            let dy = y as f32 - cy;
            let r = dx.hypot(dy);
            let inside_arch = if y <= 15 { r < 10.2 } else { true };
            if inside_arch {
                let ang = dy.atan2(dx);
                let s1 = (ang * 3.0 + r * 1.05 - t * 4.0).sin() * 0.5 + 0.5;
                let s2 = (ang * 5.0 - r * 0.72 + t * 2.4).cos() * 0.5 + 0.5;
                let swirl = s1 * 0.70 + s2 * 0.30;
                let core = (1.0 - r / 10.8).clamp(0.0, 1.0);
                let v = (0.16 + core * 0.58 + swirl * 0.44).clamp(0.14, 1.0);
                let ch = if r < 1.8 {
                    if v > 0.62 { '@' } else { '◈' }
                } else {
                    pick_char(ramp, v)
                };
                let rc = lerp(112.0, 228.0, core * core) + swirl * 42.0;
                let gc = lerp(64.0, 245.0, (core * 0.72 + swirl * 0.48).clamp(0.0, 1.0));
                let bc = 255.0;
                g.set(x as isize, y as isize, ch, rc, gc, bc, (0.16 + core * 0.42 + swirl * 0.28).clamp(0.0, 0.95));
            }
        }
    }

    // Monumental carved stone archway + keystone + glowing pillar runes
    let stone = b":;=#%H8";
    for y in 1..27 {
        for x in 3..64 {
            let dx = (x as f32 - cx) * 0.56;
            let dy = y as f32 - cy;
            let r = dx.hypot(dy);
            let in_ring = y <= 15 && (10.1..=13.2).contains(&r);
            let in_left = (9..=14).contains(&x) && (15..=26).contains(&y);
            let in_right = (52..=57).contains(&x) && (15..=26).contains(&y);
            if in_ring || in_left || in_right {
                let edge_dist = if y <= 15 {
                    (r - 10.2).abs()
                } else {
                    ((x as f32 - 14.0).abs().min((x as f32 - 52.0).abs())) * 0.56
                };
                let inner_rim = (1.0 - edge_dist / 2.6).clamp(0.0, 1.0);
                let n = hash(x as f32 * 3.0, y as f32 * 7.0);
                let is_keystone = (y == 2 || y == 3) && (x == 32 || x == 33 || x == 34);
                let is_pillar_rune = (y == 18 || y == 21 || y == 24) && (x == 11 || x == 54);
                if is_keystone {
                    g.set(x as isize, y as isize, '◈', 196.0, 240.0, 255.0, 1.15);
                } else if is_pillar_rune {
                    let pulse = 0.65 + 0.35 * (t * 3.2 + y as f32).sin();
                    g.set(x as isize, y as isize, '§', 175.0 * pulse + 45.0, 222.0 * pulse + 30.0, 255.0, 1.1);
                } else {
                    let ch = pick_char(stone, 0.32 + inner_rim * 0.45 + n * 0.25);
                    g.set(
                        x as isize,
                        y as isize,
                        ch,
                        lerp(52.0, 188.0, inner_rim),
                        lerp(58.0, 226.0, inner_rim),
                        lerp(86.0, 255.0, inner_rim),
                        inner_rim * 0.72,
                    );
                }
            }
        }
    }

    // Illuminated stepped stone dais below portal (rows 26..29)
    for y in 26..HD_ROWS {
        let w = 23.0 + (y - 26) as f32 * 2.8;
        for x in (cx - w).floor() as isize..=(cx + w).ceil() as isize {
            if x < 1 || x >= HD_COLS as isize - 1 {
                continue;
            }
            let d = (x as f32 - cx).abs() / w;
            let glow = (1.0 - d * 0.9).clamp(0.0, 1.0) * (1.0 - (y - 26) as f32 * 0.24);
            let ch = if hash(x as f32 * 5.0, y as f32 * 11.0) > 0.5 { '=' } else { '-' };
            g.set(
                x,
                y as isize,
                ch,
                lerp(48.0, 165.0, glow),
                lerp(58.0, 218.0, glow),
                lerp(88.0, 255.0, glow),
                glow * 0.65,
            );
        }
    }

    // Stardust pulled inward into the portal
    for i in 0..32 {
        let fi = i as f32;
        let a = hash(fi, 1.3) * std::f32::consts::TAU + t * 0.25;
        let p = 1.0 - (t * 0.45 + hash(fi, 2.7)).fract();
        let sx = cx + a.cos() * (10.0 + p * 21.0);
        let sy = cy + a.sin() * (5.5 + p * 11.5);
        if sx > 1.0 && sx < HD_COLS as f32 - 1.0 && ((sx - cx) * 0.56).hypot(sy - cy) > 12.8 {
            g.set(
                sx as isize,
                sy as isize,
                if p < 0.4 { '*' } else { '·' },
                188.0,
                238.0,
                255.0,
                (1.0 - p) * 1.15,
            );
        }
    }
}

// ── Scene 5: Arcane Grimoire (High-Res 80x30 Levitating Tome + Bloom Runes) ─
// 49 cols wide, exact center col 24 (anchored x=16):
const HD_BOOK: &[&str] = &[
    "     .─────────────────. .─────────────────.     ", // 0  top illuminated parchment curve
    "    /   ~~~~~~~~~~~~~   V   ~~~~~~~~~~~~~   \\    ", // 1  glowing script
    "   /   ~~~~~~~~~~~~~~~  |  ~~~~~~~~~~~~~~~   \\   ", // 2
    "  /   ~~~~~~~~~~~~~~~~  |  ~~~~~~~~~~~~~~~~   \\  ", // 3
    "  |===================\\ | /===================|  ", // 4  gilded stacked page edges
    "   \\__________________\\ | /__________________/   ", // 5  ornate leather cover
    "                       \\|/                       ", // 6  arcane clasp point
];

const HD_ALTAR: &[&str] = &[
    "          ▄▄█████████████████████████▄▄          ", // 0  carved altar cap
    "           |###%####§###◈###§####%###|           ", // 1  engraved stone frieze
    "           |##%######%#####%######%##|           ", // 2  fluted pedestal
    "         __|##%#######%###%#######%##|__         ", // 3
    "       _/======%#%======◈======%#%======\\_       ", // 4  stepped plinth
    "     _/===================================\\_     ", // 5
];

fn render_hd_grimoire(g: &mut HdGrid, t: f32) {
    g.clear();
    let bx0: isize = 8;
    let cx = bx0 as f32 + 24.0; // 32.0
    let hover: isize = if (t * 2.3).sin() > 0.35 { 0 } else { 1 };
    let book_y = 14 + hover;

    // Orbiting arcane sigil ring around the floating spellbook
    let ring_runes = ['·', '°', '◇', '+', '*', '§'];
    for i in 0..18 {
        let a = (i as f32 / 18.0) * std::f32::consts::TAU + t * 0.38;
        let rx = cx + a.cos() * 28.5;
        let ry = (book_y as f32 + 1.5) + a.sin() * 9.5;
        if rx > 1.0 && rx < HD_COLS as f32 - 1.0 && ry >= 1.0 && ry < 24.0 {
            let d = (a.sin() + 1.0) * 0.5;
            g.set(
                rx.round() as isize,
                ry.round() as isize,
                ring_runes[i % ring_runes.len()],
                168.0 + d * 82.0,
                122.0 + d * 102.0,
                232.0 + d * 23.0,
                0.45 + d * 0.65,
            );
        }
    }

    // Rising twin ribbons of amethyst & gold arcane energy with lush optical bloom!
    let runes = ['·', '+', '*', '°', '§', '◇', '◈', '^', '~'];
    for y in 1..=book_y {
        let h = (book_y - y) as f32 / 14.0;
        for strand in 0..2 {
            let phase = strand as f32 * std::f32::consts::PI;
            let sx = cx + (y as f32 * 0.44 - t * 3.2 + phase).sin() * (4.2 + h * 7.8);
            for dx in -2_isize..=2 {
                let x = sx.round() as isize + dx;
                let core = match dx.abs() {
                    0 => 1.0,
                    1 => 0.60,
                    _ => 0.28,
                };
                let v = ((1.0 - h * 0.42) * core).clamp(0.20, 1.0);
                let ch = if dx == 0 {
                    runes[((y as usize) + (t * 4.5) as usize + strand * 3) % runes.len()]
                } else if dx.abs() == 1 {
                    if hash(x as f32, y as f32) > 0.45 { '·' } else { '.' }
                } else if hash(x as f32 * 2.0, y as f32) > 0.72 {
                    '.'
                } else {
                    continue;
                };
                if strand == 0 {
                    g.set(x, y, ch, lerp(168.0, 245.0, v), lerp(104.0, 192.0, v), 255.0, v * 1.25);
                } else {
                    g.set(x, y, ch, 255.0, lerp(158.0, 236.0, v), lerp(72.0, 158.0, v), v * 1.2);
                }
            }
        }
    }

    // Levitating open spellbook
    for (ry, line) in HD_BOOK.iter().enumerate() {
        for (rx, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = bx0 + rx as isize;
            let y = book_y + ry as isize;
            if ch == '~' {
                let shimmer = 0.6 + 0.4 * (t * 4.2 + rx as f32 * 0.45).sin();
                g.set(
                    x,
                    y,
                    if shimmer > 0.82 { '=' } else { '~' },
                    228.0 * shimmer + 27.0,
                    184.0 * shimmer + 35.0,
                    255.0,
                    0.85,
                );
            } else if ry == 4 && ch == '=' {
                g.set(x, y, '=', 232.0, 202.0, 146.0, 0.45);
            } else if ry >= 5 {
                g.set(x, y, ch, 184.0, 138.0, 224.0, 0.45);
            } else {
                g.set(x, y, ch, 252.0, 236.0, 198.0, 0.72);
            }
        }
    }

    // Carved stone altar pedestal (rows 24..29)
    for (ry, line) in HD_ALTAR.iter().enumerate() {
        for (rx, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = bx0 + rx as isize;
            let y = 24 + ry as isize;
            if ch == '◈' {
                g.set(x, y, '◈', 242.0, 196.0, 255.0, 1.3);
            } else if ch == '§' {
                g.set(x, y, '§', 196.0, 156.0, 248.0, 0.85);
            } else {
                let top_light = (0.78 - ry as f32 * 0.10).clamp(0.24, 0.78);
                g.set(
                    x,
                    y,
                    ch,
                    lerp(56.0, 134.0, top_light),
                    lerp(54.0, 118.0, top_light),
                    lerp(86.0, 186.0, top_light),
                    if ry == 0 { 0.12 } else { 0.08 },
                );
            }
        }
    }
}

// ── Scene 6: Nether Portal (High-Res 80x30 Minecraft Obsidian + Bloom Vortex) ─

fn render_hd_nether(g: &mut HdGrid, t: f32) {
    g.clear();
    // Iconic Minecraft 4:5 Nether Portal proportions (28x24 chars = 196x264 px -> 0.74:1 tall!)
    let x0: isize = 19;
    let x1: isize = 46;
    let y0: isize = 1;
    let y1: isize = 24;
    let cx = (x0 + x1) as f32 * 0.5;
    let cy = (y0 + y1) as f32 * 0.5;

    // Detailed cracked basalt & glowing magma-vein floor bathed in purple portal bloom (rows 24..29)
    for y in 24..HD_ROWS {
        for x in 1..HD_COLS - 1 {
            let d = ((x as f32 - cx) * 0.30).hypot((y as f32 - 24.0) * 1.15);
            let glow = (1.0 - d / 16.0).clamp(0.0, 1.0);
            let n = hash(x as f32 * 3.0, y as f32 * 7.0);
            if n > 0.94 && glow < 0.52 {
                let pulse = 0.7 + 0.3 * (t * 3.5 + x as f32).sin();
                g.set(x as isize, y as isize, '~', 248.0 * pulse, 96.0 * pulse, 34.0 * pulse, 0.85);
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
                    lerp(48.0, 214.0, glow),
                    lerp(28.0, 88.0, glow),
                    lerp(42.0, 255.0, glow),
                    glow * 0.52,
                );
            }
        }
    }

    // Floating glowing purple Nether portal sparks drifting outward
    for i in 0..38 {
        let fi = i as f32;
        let p = (t * (0.35 + hash(fi, 1.0) * 0.35) + hash(fi, 2.0)).fract();
        let ang = hash(fi, 3.0) * std::f32::consts::TAU;
        let px = cx + ang.cos() * (12.0 + p * 22.0);
        let py = cy + ang.sin() * (8.0 + p * 10.5) - p * 2.0;
        if px > 2.0 && px < HD_COLS as f32 - 2.0 && py >= 0.0 && py < 24.5 {
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
                235.0 * fade + 20.0,
                118.0 * fade + 20.0,
                255.0 * fade,
                fade * 1.1,
            );
        }
    }

    // Rich glowing Minecraft Nether Portal vortex interior (deep violet-magenta optical bloom!)
    let swirl_chars = b"~=+*%#@8";
    for y in (y0 + 3)..=(y1 - 2) {
        for x in (x0 + 4)..=(x1 - 4) {
            let dx = (x as f32 - cx) * 0.52;
            let dy = y as f32 - cy;
            let dist = dx.hypot(dy);
            let ang = dy.atan2(dx);
            let w1 = (dist * 0.95 - t * 4.2 + ang * 2.0).sin();
            let w2 = (x as f32 * 0.32 + y as f32 * 0.42 - t * 3.1).cos();
            let v = (0.50 + w1 * 0.30 + w2 * 0.22).clamp(0.10, 1.0);
            let ch = pick_char(swirl_chars, v);
            g.set(
                x,
                y,
                ch,
                lerp(132.0, 248.0, v),
                lerp(36.0, 162.0, v * v),
                255.0,
                0.22 + v * 0.50,
            );
        }
    }

    // Detailed Minecraft Obsidian block frame (true stippled ▓ + solid █ + glowing crying-obsidian veins!)
    for y in y0..=y1 {
        for x in x0..=x1 {
            let is_top = y <= y0 + 2;
            let is_bot = y >= y1 - 1;
            let is_side = x <= x0 + 3 || x >= x1 - 3;
            let is_corner = y <= y0 + 1 && (x <= x0 + 1 || x >= x1 - 1);
            if is_corner || !(is_top || is_bot || is_side) {
                continue;
            }
            // Block grid seams (4x3 obsidian brick pattern!)
            let bx = (x - x0) / 4;
            let by = (y - y0) / 3;
            let block_hash = hash(bx as f32 * 7.0, by as f32 * 13.0);
            let n = hash(x as f32 * 5.0, y as f32 * 11.0);
            let is_seam = (x - x0) % 4 == 0 || (y - y0) % 3 == 0;
            let is_crying = block_hash > 0.78 && n > 0.62;
            let is_inner = x == x0 + 3 || x == x1 - 3 || y == y0 + 2 || y == y1 - 1;

            if is_crying {
                let pulse = 0.72 + 0.28 * (t * 4.0 + bx as f32 + by as f32).sin();
                g.set(x, y, '▓', 216.0 * pulse, 114.0 * pulse, 255.0 * pulse, 0.62);
            } else {
                let ch = if is_seam { '▒' } else if n > 0.45 { '▓' } else { '█' };
                let r = if is_inner { 86.0 } else { 46.0 } + n * 16.0;
                let gr = if is_inner { 42.0 } else { 28.0 } + n * 10.0;
                let b = if is_inner { 142.0 } else { 80.0 } + n * 24.0;
                g.set(x, y, ch, r, gr, b, if is_inner { 0.22 } else { 0.04 });
            }
        }
    }

    // Flanking flickering Nether fire braziers standing on stone plinths
    let f = 0.78 + 0.22 * (t * 7.5).sin();
    for &bx in &[x0 - 8, x1 + 8] {
        // Swirling flame tongue
        g.set(bx, y1 - 5, if f > 0.85 { '^' } else { '*' }, 255.0 * f, 204.0 * f, 96.0 * f, 1.0);
        g.set(bx - 1, y1 - 4, '^', 255.0 * f, 138.0 * f, 44.0 * f, 0.85);
        g.set(bx, y1 - 4, '#', 255.0 * f, 198.0 * f, 88.0 * f, 1.05);
        g.set(bx + 1, y1 - 4, '^', 255.0 * f, 138.0 * f, 44.0 * f, 0.85);
        g.set(bx - 1, y1 - 3, '&', 238.0 * f, 96.0 * f, 32.0 * f, 0.75);
        g.set(bx, y1 - 3, '@', 255.0 * f, 168.0 * f, 64.0 * f, 0.95);
        g.set(bx + 1, y1 - 3, '&', 238.0 * f, 96.0 * f, 32.0 * f, 0.75);
        // Stone brazier basin & pedestal
        g.put_text(bx - 2, y1 - 2, "\\###/", 102.0, 88.0, 118.0, 0.12);
        g.put_text(bx - 1, y1 - 1, "|#|", 86.0, 74.0, 102.0, 0.08);
        g.put_text(bx - 2, y1, "▄███▄", 76.0, 66.0, 92.0, 0.08);
    }
}

pub fn render_hd_scene(scene: Scene, g: &mut HdGrid, t: f32) {
    match scene {
        Scene::Eclipse => render_hd_eclipse(g, t),
        Scene::Bonfire => render_hd_bonfire(g, t),
        Scene::Torii => render_hd_torii(g, t),
        Scene::Portal => render_hd_portal(g, t),
        Scene::Grimoire => render_hd_grimoire(g, t),
        Scene::Nether => render_hd_nether(g, t),
    }
}

pub fn scene_hud_style(scene: Scene) -> (&'static str, (u8, u8, u8), (u8, u8, u8)) {
    match scene {
        Scene::Eclipse => ("◈ ECLIPSE KNIGHT", (214, 178, 108), (115, 126, 150)),
        Scene::Bonfire => ("B O N F I R E   L I T", (238, 178, 92), (138, 122, 104)),
        Scene::Torii => ("◈ SHRINE GATE", (238, 128, 134), (148, 118, 132)),
        Scene::Portal => ("◈ ASTRAL GATE", (168, 152, 255), (118, 128, 168)),
        Scene::Grimoire => ("◈ ARCANE GRIMOIRE", (204, 152, 255), (134, 118, 162)),
        Scene::Nether => ("◈ NETHER PORTAL", (216, 128, 255), (136, 112, 168)),
    }
}
