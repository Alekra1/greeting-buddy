use crate::pets::{find_pet_by_id, render_art, Rarity};
use crate::state::{PetDrop, State};
use crate::sysinfo::SysInfo;

// ── ANSI helpers ─────────────────────────────────────────────────────────────
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

pub fn rarity_color(rarity: &Rarity) -> &'static str {
    match rarity {
        Rarity::Common => "\x1b[37m",
        Rarity::Uncommon => "\x1b[32m",
        Rarity::Rare => "\x1b[34m",
        Rarity::Epic => "\x1b[35m",
        Rarity::Legendary => "\x1b[33m",
    }
}

// ── Layout constants ──────────────────────────────────────────────────────────
//
//                                            moon (col 42)       58 wide
//   info (col 1, rows 0-3)
//     pet art (col 3, row 6+)  cloud (col 19, row 6)  cloud (col 41, row 7)

const WIDTH: usize = 58;

// Moon: 13-wide × 6-tall chunky crescent, horns curling right,
// terminator shaded ▓░ down the inner edge. Keeps its own sky band.
const MOON: &[&str] = &[
    "   █████▓▓░",
    " ███▓░     ░░",
    "███▓░",
    "███▓░",
    " ██▓░░      ▓",
    "   ░▓▓███▓▓░",
];
const MOON_COL: usize = 42;
const MOON_ROW: usize = 0;

// Clouds: (row, col, sprite). They drift low beside the pet, clear of the
// info block and the moon.
#[rustfmt::skip]
const CLOUDS: &[(usize, usize, &[&str])] = &[
    (
        6,
        19,
        &[
            "         ░░░░░░",
            " ░░░   ░░░░░░░░░░",
            "░░░░░░░░░░░░░░░░░░░",
        ],
    ),
    (
        7,
        41,
        &[
            "    ░░░░",
            "  ░░░░░░░░",
            "░░░░░░░░░░░░░░░░",
        ],
    ),
];

const INFO_ROW: usize = 0;

const ART_COL: usize = 3;
const ART_ROW: usize = 6;

// Sparse stars in the open sky. Elements render over this field; stars sit
// outside the info/moon/cloud/pet zones so they stay visible on every drop.
const STARS: &[(usize, usize, char)] = &[
    (0, 39, '*'),
    (1, 32, '.'),
    (2, 29, '˚'),
    (2, 56, '˚'),
    (3, 36, '✦'),
    (4, 14, '˚'),
    (4, 30, '*'),
    (5, 22, '.'),
    (5, 38, '.'),
    (6, 40, '*'),
    (7, 55, '✦'),
    (9, 30, '*'),
];

// ── Star field ────────────────────────────────────────────────────────────────

fn star_at(row: usize, col: usize) -> Option<char> {
    STARS
        .iter()
        .find(|&&(r, c, _)| r == row && c == col)
        .map(|&(_, _, ch)| ch)
}

// ── Composited row renderer ───────────────────────────────────────────────────

/// Ink applied to a painted cell.
#[derive(Clone, Copy, PartialEq)]
enum Ink {
    Default, // terminal default foreground
    Color,   // rarity colour
    Dim,     // dimmed rarity colour
}

impl Ink {
    fn ansi(self, color: &str) -> String {
        match self {
            Ink::Default => RESET.to_string(),
            Ink::Color => color.to_string(),
            Ink::Dim => format!("{}{}", DIM, color),
        }
    }
}

#[derive(Clone)]
struct Elem {
    col: usize,
    text: String, // plain text, no ANSI
    ink: Ink,
    opaque: bool, // false → spaces let the background show through
}

/// Paint one body row: stars first, then elements in z-order (later
/// elements win cell by cell), then emit the row with run-length ANSI.
fn comp_row(elems: &[Elem], row: usize, color: &str) -> String {
    let mut cells: Vec<Option<(char, Ink)>> = (0..WIDTH)
        .map(|col| star_at(row, col).map(|ch| (ch, Ink::Dim)))
        .collect();

    for elem in elems {
        for (i, ch) in elem.text.chars().enumerate() {
            let col = elem.col + i;
            if col >= WIDTH {
                break;
            }
            if ch == ' ' {
                if elem.opaque {
                    cells[col] = None;
                }
                continue;
            }
            cells[col] = Some((ch, elem.ink));
        }
    }

    // Trailing blanks are not emitted.
    let used = cells.iter().rposition(|c| c.is_some()).map_or(0, |i| i + 1);

    let mut s = String::new();
    let mut cur: Option<Ink> = None;
    for cell in &cells[..used] {
        match cell {
            Some((ch, ink)) => {
                if cur != Some(*ink) {
                    // RESET first: a bold/dim run must not leak into the next ink.
                    if cur.is_some() && *ink != Ink::Default {
                        s.push_str(RESET);
                    }
                    s.push_str(&ink.ansi(color));
                    cur = Some(*ink);
                }
                s.push(*ch);
            }
            None => {
                if cur.is_some() {
                    s.push_str(RESET);
                    cur = None;
                }
                s.push(' ');
            }
        }
    }
    if cur.is_some() {
        s.push_str(RESET);
    }
    s
}

// ── Greeting box ─────────────────────────────────────────────────────────────

pub fn print_greeting_box(drop: &PetDrop, _state: &State, _is_new_drop: bool, si: &SysInfo) {
    let rarity = Rarity::from_str(&drop.rarity);
    let color = rarity_color(&rarity);

    let pet = match find_pet_by_id(&drop.species) {
        Some(p) => p,
        None => return,
    };
    let art = render_art(pet, &drop.eye);

    // ── Info rows at col 1: (body_row, element) ───────────────────────────
    let mut info: Vec<(usize, Elem)> = Vec::new();
    let mut ir = INFO_ROW;

    let facts = [
        ("os", &si.os),
        ("kern", &si.kernel),
        ("ip", &si.local_ip),
        ("pkgs", &si.packages),
    ];
    for (label, value) in facts {
        info.push((
            ir,
            Elem {
                col: 1,
                text: format!("{:<8}", label),
                ink: Ink::Dim,
                opaque: true,
            },
        ));
        info.push((
            ir,
            Elem {
                col: 9,
                text: value.clone(),
                ink: Ink::Default,
                opaque: true,
            },
        ));
        ir += 1;
    }

    let total_rows = (ART_ROW + art.len())
        .max(info.iter().map(|(r, _)| r + 1).max().unwrap_or(0))
        .max(MOON_ROW + MOON.len());

    // ── Render ────────────────────────────────────────────────────────────
    for row in 0..total_rows {
        // Painted back-to-front: moon, clouds, pet art (all transparent), then info.
        let mut elems: Vec<Elem> = Vec::new();

        if row >= MOON_ROW && row < MOON_ROW + MOON.len() {
            elems.push(Elem {
                col: MOON_COL,
                text: MOON[row - MOON_ROW].to_string(),
                ink: Ink::Dim,
                opaque: false,
            });
        }

        for &(cloud_row, cloud_col, sprite) in CLOUDS {
            if row >= cloud_row && row < cloud_row + sprite.len() {
                elems.push(Elem {
                    col: cloud_col,
                    text: sprite[row - cloud_row].to_string(),
                    ink: Ink::Color,
                    opaque: false,
                });
            }
        }

        if row >= ART_ROW && row < ART_ROW + art.len() {
            elems.push(Elem {
                col: ART_COL,
                text: art[row - ART_ROW].clone(),
                ink: Ink::Color,
                opaque: false,
            });
        }

        for (info_row, elem) in &info {
            if *info_row == row {
                elems.push((*elem).clone());
            }
        }

        println!("{}", comp_row(&elems, row, color));
    }

    println!();
}

// ── Collection view ───────────────────────────────────────────────────────────

pub fn print_collection(state: &State) {
    if state.drops.is_empty() {
        println!("  No pets collected yet. Open a terminal to get your first drop!");
        println!();
        return;
    }

    let default_id = state.default_drop_id.as_deref().unwrap_or("");

    println!(
        "  {:<3} {:<22} {:<12} {:<10} {}",
        "#", "Name", "Species", "Rarity", "Eye"
    );
    println!("  {}", "─".repeat(58));

    for (i, drop) in state.drops.iter().enumerate() {
        let rarity = Rarity::from_str(&drop.rarity);
        let color = rarity_color(&rarity);
        let default_marker = if drop.id == default_id {
            format!("  {}[default]{}", "\x1b[36m", RESET)
        } else {
            String::new()
        };
        let num = format!("{}{}{}", DIM, i + 1, RESET);
        let rstr = format!("{}{} {}{}", color, rarity.stars(), drop.rarity, RESET);
        println!(
            "  {:<12} {:<22} {:<12} {:<24} {}{}",
            num, drop.pet_name, drop.species, rstr, drop.eye, default_marker,
        );
    }
    println!();
    println!(
        "  {}Use `greeting set <number>` to set a default.{}",
        DIM, RESET
    );
    println!();
}

// ── Catalog view ──────────────────────────────────────────────────────────────

pub fn print_catalog(state: &State) {
    use crate::pets::catalog;

    let total = catalog().len();
    let discovered = catalog()
        .iter()
        .filter(|p| state.owns_species(p.id))
        .count();
    println!(
        "  {}{}/{} species discovered{}",
        DIM, discovered, total, RESET
    );
    println!();

    for pet in catalog() {
        let count = state.species_count(pet.id);
        if count > 0 {
            println!(
                "  {:<12}  {} drop{}",
                pet.name,
                count,
                if count == 1 { "" } else { "s" }
            );
        } else {
            println!("  {}???{}", DIM, RESET);
        }
    }
    println!();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

