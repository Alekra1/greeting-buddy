use crate::pets::{find_pet_by_id, render_art, Rarity};
use crate::state::{PetDrop, State};
use crate::sysinfo::SysInfo;

// ── ANSI helpers ─────────────────────────────────────────────────────────────
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
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
//  ┌──────────────────────────────────────────────────┐  52 wide
//  │* stars scattered everywhere in empty space  *    │
//  │  info (col 1)      moon (col 25)                 │
//  │  ...               art  (col 33, row 3+)         │
//  └──────────────────────────────────────────────────┘

const OUTER_W: usize = 52;
const INNER_W: usize = 50;

// Moon: 5-wide × 4-tall thin C-crescent with ░▒▓ gradient.
const MOON: &[&str] = &[
    " █▓▒░", // top arc
    "█▓▒  ", // C mouth top
    "█▓▒  ", // C mouth bottom
    " █▓▒░", // bottom arc
];
const MOON_COL: usize = 22;
const MOON_ROW: usize = 0;

const ART_COL: usize = 33;
const ART_ROW: usize = 3;

// Stars spread through the full greeting body. Elements render over this field.
const STARS: &[(usize, usize, char)] = &[
    (0, 4, '*'),
    (0, 18, '.'),
    (0, 29, '✦'),
    (0, 42, '.'),
    (0, 48, ','),
    (1, 9, '☾'),
    (1, 19, '.'),
    (1, 31, '*'),
    (1, 39, '˚'),
    (2, 3, '.'),
    (2, 14, '✦'),
    (2, 28, ','),
    (2, 44, '.'),
    (3, 6, '˚'),
    (3, 20, '.'),
    (3, 30, '✮'),
    (3, 47, '*'),
    (4, 12, '*'),
    (4, 24, '.'),
    (4, 37, '✩'),
    (5, 2, '.'),
    (5, 16, '✴'),
    (5, 27, '˚'),
    (5, 43, '.'),
    (6, 8, '*'),
    (6, 21, '.'),
    (6, 32, '★'),
    (6, 48, ','),
    (7, 5, '˚'),
    (7, 18, '.'),
    (7, 35, '✦'),
    (7, 45, '*'),
    (8, 10, '.'),
    (8, 25, '*'),
    (8, 38, 'ﾟ'),
    (8, 49, '.'),
];

// ── Border ───────────────────────────────────────────────────────────────────

fn box_top(color: &str) -> String {
    format!("{}┌{}┐{}", color, "─".repeat(OUTER_W - 2), RESET)
}

fn box_bottom(color: &str) -> String {
    format!("{}└{}┘{}", color, "─".repeat(OUTER_W - 2), RESET)
}

// ── Star field ────────────────────────────────────────────────────────────────

fn star_at(row: usize, col: usize) -> Option<char> {
    STARS
        .iter()
        .find(|&&(r, c, _)| r == row && c == col)
        .map(|&(_, _, ch)| ch)
}

// ── Composited row renderer ───────────────────────────────────────────────────

struct Elem {
    col: usize,
    text: String, // may contain ANSI codes; ends with RESET
    vlen: usize,  // visual (non-ANSI) length
}

/// Render a single body row. Elements are placed at absolute columns over a
/// deterministic star background; gaps between elements show stars or spaces.
fn comp_row(mut elems: Vec<Elem>, row: usize, color: &str) -> String {
    elems.sort_by_key(|e| e.col);
    let mut s = String::new();
    s.push_str(color);
    s.push('│');
    s.push_str(RESET);
    let mut col = 0usize;
    for elem in &elems {
        let target = elem.col.min(INNER_W);
        while col < target {
            if let Some(ch) = star_at(row, col) {
                s.push_str(DIM);
                s.push_str(color);
                s.push(ch);
                s.push_str(RESET);
            } else {
                s.push(' ');
            }
            col += 1;
        }
        if col == elem.col && col + elem.vlen <= INNER_W {
            s.push_str(&elem.text);
            col += elem.vlen;
        }
    }
    while col < INNER_W {
        if let Some(ch) = star_at(row, col) {
            s.push_str(DIM);
            s.push_str(color);
            s.push(ch);
            s.push_str(RESET);
        } else {
            s.push(' ');
        }
        col += 1;
    }
    s.push_str(color);
    s.push('│');
    s.push_str(RESET);
    s
}

// ── Greeting box ─────────────────────────────────────────────────────────────

pub fn print_greeting_box(drop: &PetDrop, state: &State, is_new_drop: bool, si: &SysInfo) {
    let rarity = Rarity::from_str(&drop.rarity);
    let color = rarity_color(&rarity);

    let pet = match find_pet_by_id(&drop.species) {
        Some(p) => p,
        None => return,
    };
    let art = render_art(pet, &drop.eye);

    // ── Info lines at col 1 ───────────────────────────────────────────────
    // Each entry: (body_row_index, styled_string)
    let mut info: Vec<(usize, String)> = Vec::new();
    let mut ir = 0usize;

    info.push((ir, format!("{}{}{}{}", BOLD, color, drop.pet_name, RESET)));
    ir += 1;
    if is_new_drop {
        info.push((ir, format!("{}{}✦ New drop! ✦{}", BOLD, color, RESET)));
        ir += 1;
    }
    info.push((
        ir,
        format!("{}{} {}{}", color, rarity.stars(), drop.rarity, RESET),
    ));
    ir += 1;
    let sc = state.species_count(&drop.species);
    if sc > 1 {
        info.push((
            ir,
            format!(
                "{}#{} of {} {}s{}",
                DIM,
                drop_number(drop, state),
                sc,
                pet.name,
                RESET,
            ),
        ));
        ir += 1;
    }
    info.push((ir, format!("{}{}──────────────────{}", DIM, color, RESET)));
    ir += 1;
    let lc = format!("{}{}", DIM, color);
    info.push((ir, format!("{}os{}      {}", lc, RESET, si.os)));
    ir += 1;
    info.push((ir, format!("{}kern{}    {}", lc, RESET, si.kernel)));
    ir += 1;
    info.push((ir, format!("{}ip{}      {}", lc, RESET, si.local_ip)));
    ir += 1;
    info.push((ir, format!("{}pkgs{}    {}", lc, RESET, si.packages)));
    ir += 1;
    let _ = ir;

    let total_rows = (ART_ROW + art.len())
        .max(info.iter().map(|(r, _)| r + 1).max().unwrap_or(0))
        .max(MOON_ROW + MOON.len());

    // ── Render ────────────────────────────────────────────────────────────
    println!("{}", box_top(color));

    for row in 0..total_rows {
        let mut elems: Vec<Elem> = Vec::new();

        // Info
        for (info_row, text) in &info {
            if *info_row == row {
                let vl = visual_len(text);
                elems.push(Elem {
                    col: 1,
                    text: text.clone(),
                    vlen: vl,
                });
            }
        }

        // Moon (dim)
        if row >= MOON_ROW && row < MOON_ROW + MOON.len() {
            let ml = MOON[row - MOON_ROW];
            elems.push(Elem {
                col: MOON_COL,
                text: format!("{}{}{}{}", DIM, color, ml, RESET),
                vlen: visual_len(ml),
            });
        }

        // Art
        if row >= ART_ROW && row < ART_ROW + art.len() {
            let al = &art[row - ART_ROW];
            let vl = visual_len(al);
            elems.push(Elem {
                col: ART_COL,
                text: format!("{}{}{}", color, al, RESET),
                vlen: vl,
            });
        }

        println!("{}", comp_row(elems, row, color));
    }

    println!("{}", box_bottom(color));
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

fn drop_number(drop: &PetDrop, state: &State) -> usize {
    state
        .drops
        .iter()
        .filter(|d| d.species == drop.species)
        .position(|d| d.id == drop.id)
        .map(|i| i + 1)
        .unwrap_or(1)
}

fn visual_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }
    len
}
