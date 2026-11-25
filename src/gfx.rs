use std::fmt::Write as _;

use crate::font_atlas::{glyph_bitmap, GLYPH_H, GLYPH_W};

pub const HD_COLS: usize = 66;
pub const HD_ROWS: usize = 30;
pub const IMG_W: usize = HD_COLS * GLYPH_W; // 462 px
pub const IMG_H: usize = HD_ROWS * GLYPH_H; // 330 px

const HALF_W: usize = IMG_W / 2; // 231 px
const HALF_H: usize = IMG_H / 2; // 165 px

pub const KITTY_IMAGE_ID: u32 = 4201;

#[link(name = "z")]
extern "C" {
    fn compress2(
        dest: *mut u8,
        dest_len: *mut libc::c_ulong,
        source: *const u8,
        source_len: libc::c_ulong,
        level: libc::c_int,
    ) -> libc::c_int;
    fn compressBound(source_len: libc::c_ulong) -> libc::c_ulong;
}

fn zlib_compress_fast(input: &[u8], out: &mut Vec<u8>) -> bool {
    unsafe {
        let bound = compressBound(input.len() as libc::c_ulong) as usize;
        out.resize(bound, 0);
        let mut dest_len = bound as libc::c_ulong;
        let rc = compress2(
            out.as_mut_ptr(),
            &mut dest_len,
            input.as_ptr(),
            input.len() as libc::c_ulong,
            1, // Z_BEST_SPEED (~1ms per frame)
        );
        if rc == 0 {
            out.truncate(dest_len as usize);
            true
        } else {
            out.clear();
            false
        }
    }
}

const B64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8], out: &mut String) {
    out.clear();
    out.reserve((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(B64_TABLE[((n >> 18) & 63) as usize] as char);
        out.push(B64_TABLE[((n >> 12) & 63) as usize] as char);
        out.push(B64_TABLE[((n >> 6) & 63) as usize] as char);
        out.push(B64_TABLE[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(B64_TABLE[((n >> 18) & 63) as usize] as char);
        out.push(B64_TABLE[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(B64_TABLE[((n >> 18) & 63) as usize] as char);
        out.push(B64_TABLE[((n >> 12) & 63) as usize] as char);
        out.push(B64_TABLE[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
}

#[derive(Clone, Copy)]
pub struct HdCell {
    pub ch: char,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub glow: f32,
}

impl Default for HdCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            r: 0,
            g: 0,
            b: 0,
            glow: 0.0,
        }
    }
}

pub struct HdGrid {
    pub cells: Vec<HdCell>,
}

impl HdGrid {
    pub fn new() -> Self {
        Self {
            cells: vec![HdCell::default(); HD_COLS * HD_ROWS],
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(HdCell::default());
    }

    #[inline]
    pub fn set(&mut self, x: isize, y: isize, ch: char, r: f32, g: f32, b: f32, glow: f32) {
        if x < 0 || x >= HD_COLS as isize || y < 0 || y >= HD_ROWS as isize {
            return;
        }
        self.cells[y as usize * HD_COLS + x as usize] = HdCell {
            ch,
            r: r.clamp(0.0, 255.0) as u8,
            g: g.clamp(0.0, 255.0) as u8,
            b: b.clamp(0.0, 255.0) as u8,
            glow: glow.clamp(0.0, 1.5),
        };
    }

    pub fn put_text(&mut self, x: isize, y: isize, text: &str, r: f32, g: f32, b: f32, glow: f32) {
        for (i, ch) in text.chars().enumerate() {
            if ch == ' ' {
                self.set(x + i as isize, y, ' ', 0.0, 0.0, 0.0, 0.0);
            } else {
                self.set(x + i as isize, y, ch, r, g, b, glow);
            }
        }
    }
}

/// Fast separable horizontal + vertical box blur (3 passes approximates Gaussian tightly).
fn box_blur_rgb(src: &mut [f32], tmp: &mut [f32], w: usize, h: usize, radius: usize) {
    if radius == 0 {
        return;
    }
    let inv = 1.0 / (2 * radius + 1) as f32;

    // Horizontal pass: src -> tmp
    for y in 0..h {
        let row = y * w * 3;
        let mut sr = src[row] * (radius + 1) as f32;
        let mut sg = src[row + 1] * (radius + 1) as f32;
        let mut sb = src[row + 2] * (radius + 1) as f32;
        for i in 1..=radius {
            let x = i.min(w - 1);
            let idx = row + x * 3;
            sr += src[idx];
            sg += src[idx + 1];
            sb += src[idx + 2];
        }
        for x in 0..w {
            let out_idx = row + x * 3;
            tmp[out_idx] = sr * inv;
            tmp[out_idx + 1] = sg * inv;
            tmp[out_idx + 2] = sb * inv;

            let rem_x = x.saturating_sub(radius);
            let add_x = (x + radius + 1).min(w - 1);
            let r_idx = row + rem_x * 3;
            let a_idx = row + add_x * 3;
            sr += src[a_idx] - src[r_idx];
            sg += src[a_idx + 1] - src[r_idx + 1];
            sb += src[a_idx + 2] - src[r_idx + 2];
        }
    }

    // Vertical pass: tmp -> src
    for x in 0..w {
        let col = x * 3;
        let mut sr = tmp[col] * (radius + 1) as f32;
        let mut sg = tmp[col + 1] * (radius + 1) as f32;
        let mut sb = tmp[col + 2] * (radius + 1) as f32;
        for i in 1..=radius {
            let y = i.min(h - 1);
            let idx = y * w * 3 + col;
            sr += tmp[idx];
            sg += tmp[idx + 1];
            sb += tmp[idx + 2];
        }
        for y in 0..h {
            let out_idx = y * w * 3 + col;
            src[out_idx] = sr * inv;
            src[out_idx + 1] = sg * inv;
            src[out_idx + 2] = sb * inv;

            let rem_y = y.saturating_sub(radius);
            let add_y = (y + radius + 1).min(h - 1);
            let r_idx = rem_y * w * 3 + col;
            let a_idx = add_y * w * 3 + col;
            sr += tmp[a_idx] - tmp[r_idx];
            sg += tmp[a_idx + 1] - tmp[r_idx + 1];
            sb += tmp[a_idx + 2] - tmp[r_idx + 2];
        }
    }
}

pub struct BloomRenderer {
    fg_rgb: Vec<f32>,  // premultiplied sharp glyph RGB (IMG_W * IMG_H * 3)
    fg_a: Vec<f32>,    // sharp glyph alpha (IMG_W * IMG_H)
    tight: Vec<f32>,   // half-res tight glow (HALF_W * HALF_H * 3)
    wide: Vec<f32>,    // half-res wide optical bloom (HALF_W * HALF_H * 3)
    tmp: Vec<f32>,     // scratch blur buffer
    pub rgba: Vec<u8>, // final straight RGBA8888 output (IMG_W * IMG_H * 4)
    zbuf: Vec<u8>,
    b64: String,
}

impl BloomRenderer {
    pub fn new() -> Self {
        Self {
            fg_rgb: vec![0.0; IMG_W * IMG_H * 3],
            fg_a: vec![0.0; IMG_W * IMG_H],
            tight: vec![0.0; HALF_W * HALF_H * 3],
            wide: vec![0.0; HALF_W * HALF_H * 3],
            tmp: vec![0.0; HALF_W * HALF_H * 3],
            rgba: vec![0u8; IMG_W * IMG_H * 4],
            zbuf: Vec::with_capacity(65536),
            b64: String::with_capacity(90000),
        }
    }

    pub fn rasterize(&mut self, grid: &HdGrid) {
        self.fg_rgb.fill(0.0);
        self.fg_a.fill(0.0);
        self.tight.fill(0.0);

        // 1. Blit all 80x30 tiny antialiased JetBrainsMono glyphs & seed half-res glow buffer
        for gy in 0..HD_ROWS {
            let py0 = gy * GLYPH_H;
            let row_off = gy * HD_COLS;
            for gx in 0..HD_COLS {
                let c = grid.cells[row_off + gx];
                if c.ch == ' ' {
                    continue;
                }
                let bmp = glyph_bitmap(c.ch);
                let px0 = gx * GLYPH_W;
                let r = c.r as f32 * (1.0 / 255.0);
                let g = c.g as f32 * (1.0 / 255.0);
                let b = c.b as f32 * (1.0 / 255.0);
                let glow = c.glow;

                for by in 0..GLYPH_H {
                    let y = py0 + by;
                    let hy = y >> 1;
                    let b_row = by * GLYPH_W;
                    let f_row = y * IMG_W;
                    let h_row = hy * HALF_W;
                    for bx in 0..GLYPH_W {
                        let a_u8 = bmp[b_row + bx];
                        if a_u8 == 0 {
                            continue;
                        }
                        let a = a_u8 as f32 * (1.0 / 255.0);
                        let x = px0 + bx;
                        let fi = f_row + x;
                        self.fg_a[fi] = a;
                        let fi3 = fi * 3;
                        self.fg_rgb[fi3] = r * a;
                        self.fg_rgb[fi3 + 1] = g * a;
                        self.fg_rgb[fi3 + 2] = b * a;

                        if glow > 0.08 {
                            let hi3 = (h_row + (x >> 1)) * 3;
                            let w = a * glow * 0.42;
                            self.tight[hi3] += r * w;
                            self.tight[hi3 + 1] += g * w;
                            self.tight[hi3 + 2] += b * w;
                        }
                    }
                }
            }
        }

        // 2. Copy glow seed into wide buffer, then run 2-scale separable Gaussian bloom!
        self.wide.copy_from_slice(&self.tight);

        // Tight inner neon halo (radius 2 at half-res = ~4px crisp glow around bright letters)
        box_blur_rgb(&mut self.tight, &mut self.tmp, HALF_W, HALF_H, 2);
        box_blur_rgb(&mut self.tight, &mut self.tmp, HALF_W, HALF_H, 2);

        // Lush wide optical bloom (radius 8 at half-res = ~16px atmospheric glow like ansenjeo!)
        box_blur_rgb(&mut self.wide, &mut self.tmp, HALF_W, HALF_H, 7);
        box_blur_rgb(&mut self.wide, &mut self.tmp, HALF_W, HALF_H, 7);
        box_blur_rgb(&mut self.wide, &mut self.tmp, HALF_W, HALF_H, 6);

        // 3. Composite crisp foreground glyphs + bilinear-sampled 2-scale bloom into straight RGBA
        for y in 0..IMG_H {
            let hy0 = (y >> 1).min(HALF_H - 1);
            let hy1 = (hy0 + 1).min(HALF_H - 1);
            let fy = if (y & 1) == 0 { 0.25_f32 } else { 0.75_f32 };
            let f_row = y * IMG_W;

            for x in 0..IMG_W {
                let hx0 = (x >> 1).min(HALF_W - 1);
                let hx1 = (hx0 + 1).min(HALF_W - 1);
                let fx = if (x & 1) == 0 { 0.25_f32 } else { 0.75_f32 };

                let w00 = (1.0 - fx) * (1.0 - fy);
                let w10 = fx * (1.0 - fy);
                let w01 = (1.0 - fx) * fy;
                let w11 = fx * fy;

                let i00 = (hy0 * HALF_W + hx0) * 3;
                let i10 = (hy0 * HALF_W + hx1) * 3;
                let i01 = (hy1 * HALF_W + hx0) * 3;
                let i11 = (hy1 * HALF_W + hx1) * 3;

                let tr = self.tight[i00] * w00 + self.tight[i10] * w10 + self.tight[i01] * w01 + self.tight[i11] * w11;
                let tg = self.tight[i00 + 1] * w00 + self.tight[i10 + 1] * w10 + self.tight[i01 + 1] * w01 + self.tight[i11 + 1] * w11;
                let tb = self.tight[i00 + 2] * w00 + self.tight[i10 + 2] * w10 + self.tight[i01 + 2] * w01 + self.tight[i11 + 2] * w11;

                let wr = self.wide[i00] * w00 + self.wide[i10] * w10 + self.wide[i01] * w01 + self.wide[i11] * w11;
                let wg = self.wide[i00 + 1] * w00 + self.wide[i10 + 1] * w10 + self.wide[i01 + 1] * w01 + self.wide[i11 + 1] * w11;
                let wb = self.wide[i00 + 2] * w00 + self.wide[i10 + 2] * w10 + self.wide[i01 + 2] * w01 + self.wide[i11 + 2] * w11;

                let fi = f_row + x;
                let fi3 = fi * 3;
                let fa = self.fg_a[fi];

                // Additive HDR bloom with soft-knee tone compression so neon halos stay rich & saturated
                let br = tr * 0.92 + wr * 1.15;
                let bg = tg * 0.92 + wg * 1.15;
                let bb = tb * 0.92 + wb * 1.15;
                let bmax = br.max(bg).max(bb);
                let tone = if bmax > 0.0001 {
                    1.0 / (1.0 + bmax * 0.65)
                } else {
                    1.0
                };
                let lr = (self.fg_rgb[fi3] + br * tone).min(1.0);
                let lg = (self.fg_rgb[fi3 + 1] + bg * tone).min(1.0);
                let lb = (self.fg_rgb[fi3 + 2] + bb * tone).min(1.0);

                let bloom_a = lr.max(lg).max(lb);
                let out_a = fa.max(bloom_a).clamp(0.0, 1.0);

                let oi = fi * 4;
                if out_a < 0.004 {
                    self.rgba[oi] = 0;
                    self.rgba[oi + 1] = 0;
                    self.rgba[oi + 2] = 0;
                    self.rgba[oi + 3] = 0;
                } else {
                    let inv_a = 1.0 / out_a;
                    self.rgba[oi] = ((lr * inv_a).min(1.0) * 255.0) as u8;
                    self.rgba[oi + 1] = ((lg * inv_a).min(1.0) * 255.0) as u8;
                    self.rgba[oi + 2] = ((lb * inv_a).min(1.0) * 255.0) as u8;
                    self.rgba[oi + 3] = (out_a * 255.0) as u8;
                }
            }
        }
    }

    /// Emit zlib-compressed Kitty Graphics Protocol chunked payload placing image at current cursor.
    /// Uses `C=1` (no cursor movement), `q=2` (suppress responses), and fixed `i=4201,p=1` for
    /// zero-flicker in-place animation replacement in Ghostty / Kitty / WezTerm!
    pub fn emit_kitty_graphics(&mut self, out: &mut String, cols: usize, rows: usize) -> bool {
        if !zlib_compress_fast(&self.rgba, &mut self.zbuf) {
            return false;
        }
        base64_encode(&self.zbuf, &mut self.b64);

        let bytes = self.b64.as_bytes();
        let chunk_size = 4096;
        let mut offset = 0;
        let mut first = true;

        while offset < bytes.len() {
            let end = (offset + chunk_size).min(bytes.len());
            let more = if end < bytes.len() { 1 } else { 0 };
            let chunk = &self.b64[offset..end];

            if first {
                let _ = write!(
                    out,
                    "\x1b_Ga=T,f=32,o=z,s={},v={},c={},r={},i={},p=1,C=1,q=2,m={};{}\x1b\\",
                    IMG_W, IMG_H, cols, rows, KITTY_IMAGE_ID, more, chunk
                );
                first = false;
            } else {
                let _ = write!(out, "\x1b_Gm={};{}\x1b\\", more, chunk);
            }
            offset = end;
        }
        true
    }

    /// Emit native SIXEL graphics (`P2=1` transparent background) for terminals like `foot`,
    /// `mlterm`, `contour`, etc. Uses 4x4 Bayer ordered dithering on bloom alpha so soft
    /// outer halos dissolve cleanly into transparent/blurred terminal backgrounds with zero box!
    pub fn emit_sixel(&self, out: &mut String, dst_w: usize, dst_h_raw: usize) {
        let dst_w = dst_w.max(60);
        let dst_h = (dst_h_raw.max(36) / 6) * 6;
        if dst_h == 0 {
            return;
        }

        const BAYER4: [u8; 16] = [
            0, 8, 2, 10,
            12, 4, 14, 6,
            3, 11, 1, 9,
            15, 7, 13, 5,
        ];

        // 1. Sample RGBA -> 12-bit (4x4x4) exact-average adaptive palette (preserves true warm/cool
        //    hues cleanly with zero olive-green shift or blotches; typically 70-160 colors/frame!)
        let n_px = dst_w * dst_h;
        let mut bucket_buf = vec![0u16; n_px]; // 0 = transparent, 1..=4096 = 12-bit bucket + 1
        let mut counts = [0u32; 4096];
        let mut sum_r = [0u64; 4096];
        let mut sum_g = [0u64; 4096];
        let mut sum_b = [0u64; 4096];
        let exact_1to1 = dst_w == IMG_W && dst_h == IMG_H;

        for dy in 0..dst_h {
            let sy = if exact_1to1 {
                dy
            } else {
                ((dy * IMG_H) / dst_h).min(IMG_H - 1)
            };
            let bayer_row = (dy & 3) * 4;
            let dst_row = dy * dst_w;
            let src_row = sy * IMG_W;

            for dx in 0..dst_w {
                let sx = if exact_1to1 {
                    dx
                } else {
                    ((dx * IMG_W) / dst_w).min(IMG_W - 1)
                };
                let si = (src_row + sx) * 4;
                let a = self.rgba[si + 3];
                if a < 18 {
                    continue;
                }
                let af = a as f32 * (1.0 / 255.0);
                // Solid glyphs & inner glow stay 100% opaque; soft outer bloom feathers smoothly
                let thresh = (BAYER4[bayer_row + (dx & 3)] as f32 + 0.5) * (1.0 / 16.0);
                if af < 0.68 && af < thresh * 0.68 {
                    continue;
                }

                // Scale bloom brightness smoothly with alpha while preserving exact R:G:B hue ratio
                let shade = af.powf(0.78);
                let r = (self.rgba[si] as f32 * shade).min(255.0) as u32;
                let g = (self.rgba[si + 1] as f32 * shade).min(255.0) as u32;
                let b = (self.rgba[si + 2] as f32 * shade).min(255.0) as u32;
                if r < 6 && g < 6 && b < 6 {
                    continue;
                }

                let bidx = (((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4)) as usize;
                counts[bidx] += 1;
                sum_r[bidx] += r as u64;
                sum_g[bidx] += g as u64;
                sum_b[bidx] += b as u64;
                bucket_buf[dst_row + dx] = (bidx + 1) as u16;
            }
        }

        // Collect active buckets sorted by frequency, cap at 240 Sixel palette entries
        let mut active: Vec<(usize, u32)> = counts
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| if c > 0 { Some((i, c)) } else { None })
            .collect();
        if active.len() > 240 {
            active.sort_unstable_by(|a, b| b.1.cmp(&a.1));
            active.truncate(240);
        }

        let mut bucket_to_pal = [0u8; 4096];
        let mut pal_rgb: Vec<(u8, u8, u8)> = Vec::with_capacity(active.len());
        for (slot, &(bidx, cnt)) in active.iter().enumerate() {
            let pid = (slot + 1) as u8;
            bucket_to_pal[bidx] = pid;
            let ar = (sum_r[bidx] / cnt as u64).min(255) as u8;
            let ag = (sum_g[bidx] / cnt as u64).min(255) as u8;
            let ab = (sum_b[bidx] / cnt as u64).min(255) as u8;
            pal_rgb.push((ar, ag, ab));
        }

        // Map any rare overflow buckets (>240) to nearest palette entry
        if active.len() == 240 {
            for bidx in 0..4096 {
                if counts[bidx] > 0 && bucket_to_pal[bidx] == 0 {
                    let ar = (sum_r[bidx] / counts[bidx] as u64) as i32;
                    let ag = (sum_g[bidx] / counts[bidx] as u64) as i32;
                    let ab = (sum_b[bidx] / counts[bidx] as u64) as i32;
                    let mut best_id = 1u8;
                    let mut best_d = i32::MAX;
                    for (slot, &(pr, pg, pb)) in pal_rgb.iter().enumerate() {
                        let dr = ar - pr as i32;
                        let dg = ag - pg as i32;
                        let db = ab - pb as i32;
                        let d = dr * dr + dg * dg + db * db;
                        if d < best_d {
                            best_d = d;
                            best_id = (slot + 1) as u8;
                        }
                    }
                    bucket_to_pal[bidx] = best_id;
                }
            }
        }

        let mut idx_buf = vec![0u8; n_px];
        for i in 0..n_px {
            let b = bucket_buf[i];
            if b != 0 {
                idx_buf[i] = bucket_to_pal[(b - 1) as usize];
            }
        }

        // 2. DCS Sixel header: P1=0, P2=1 (transparent background!), P3=0, raster 1:1 WxH
        let _ = write!(out, "\x1bP0;1;0q\"1;1;{};{}", dst_w, dst_h);

        // 3. Emit exact-average adaptive palette entries (0..=100% per channel)
        for (slot, &(r, g, b)) in pal_rgb.iter().enumerate() {
            let rp = (r as u32 * 100 + 127) / 255;
            let gp = (g as u32 * 100 + 127) / 255;
            let bp = (b as u32 * 100 + 127) / 255;
            let _ = write!(out, "#{};2;{};{};{}", slot + 1, rp, gp, bp);
        }

        // 4. Encode 6-row Sixel bands with RLE
        let mut band_colors = Vec::with_capacity(64);
        let mut band_seen = [false; 241];
        let mut mask_row = vec![0u8; dst_w];

        for band_y in (0..dst_h).step_by(6) {
            band_colors.clear();
            band_seen.fill(false);

            for row_off in 0..6 {
                let row = (band_y + row_off) * dst_w;
                for x in 0..dst_w {
                    let c = idx_buf[row + x] as usize;
                    if c != 0 && !band_seen[c] {
                        band_seen[c] = true;
                        band_colors.push(c as u8);
                    }
                }
            }

            for (ci, &color) in band_colors.iter().enumerate() {
                let mut last_nz = 0usize;
                for x in 0..dst_w {
                    let mut bits = 0u8;
                    for row_off in 0..6 {
                        if idx_buf[(band_y + row_off) * dst_w + x] == color {
                            bits |= 1 << row_off;
                        }
                    }
                    mask_row[x] = bits;
                    if bits != 0 {
                        last_nz = x + 1;
                    }
                }
                if last_nz == 0 {
                    continue;
                }

                let _ = write!(out, "#{}", color);
                let mut x = 0;
                while x < last_nz {
                    let b = mask_row[x];
                    let mut run = 1;
                    while x + run < last_nz && mask_row[x + run] == b {
                        run += 1;
                    }
                    let ch = (b + 63) as char;
                    if run >= 4 {
                        let _ = write!(out, "!{}{}", run, ch);
                    } else {
                        for _ in 0..run {
                            out.push(ch);
                        }
                    }
                    x += run;
                }

                if ci + 1 < band_colors.len() {
                    out.push('$');
                }
            }

            if band_y + 6 < dst_h {
                out.push('-');
            }
        }

        out.push_str("\x1b\\");
    }
}

pub fn emit_kitty_delete(out: &mut String) {
    let _ = write!(out, "\x1b_Ga=d,d=i,i={},q=2\x1b\\", KITTY_IMAGE_ID);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GraphicsProto {
    Kitty,
    Sixel,
    Text,
}

fn parent_pid_of(pid: i32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let rparen = stat.rfind(')')?;
    let after = stat.get(rparen + 2..)?;
    let mut parts = after.split_whitespace();
    let _state = parts.next()?;
    let ppid = parts.next()?.parse::<i32>().ok()?;
    if ppid > 1 {
        Some(ppid)
    } else {
        None
    }
}

fn detect_from_ancestors(start_pid: i32) -> Option<GraphicsProto> {
    let mut pid = start_pid;
    for _ in 0..10 {
        if let Ok(comm) = std::fs::read_to_string(format!("/proc/{}/comm", pid)) {
            let name = comm.trim().to_ascii_lowercase();
            if name.contains("ghostty") || name == "kitty" || name.starts_with("wezterm") {
                return Some(GraphicsProto::Kitty);
            }
            if name == "foot" || name == "footclient" || name == "mlterm" || name == "contour" {
                return Some(GraphicsProto::Sixel);
            }
        }
        pid = parent_pid_of(pid)?;
    }
    None
}

pub fn detect_graphics_proto(hint_pid: Option<i32>) -> GraphicsProto {
    if std::env::var("TMUX").is_ok() || std::env::var("STY").is_ok() {
        return GraphicsProto::Text;
    }
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() || std::env::var("KITTY_WINDOW_ID").is_ok() {
        return GraphicsProto::Kitty;
    }
    if let Ok(tp) = std::env::var("TERM_PROGRAM") {
        let lower = tp.to_ascii_lowercase();
        if lower.contains("ghostty") || lower.contains("kitty") || lower.contains("wezterm") {
            return GraphicsProto::Kitty;
        }
        if lower.contains("foot") || lower.contains("contour") {
            return GraphicsProto::Sixel;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        let lower = term.to_ascii_lowercase();
        if lower.contains("ghostty") || lower.contains("kitty") {
            return GraphicsProto::Kitty;
        }
        if lower.contains("foot") || lower.contains("sixel") || lower.contains("mlterm") {
            return GraphicsProto::Sixel;
        }
    }
    let start_pid = hint_pid.unwrap_or_else(|| std::process::id() as i32);
    if let Some(proto) = detect_from_ancestors(start_pid) {
        return proto;
    }
    GraphicsProto::Text
}
