use crate::pets::{find_pet_by_id, hat_line_for, render_art, Rarity};
use crate::state::{PetDrop, State};

// ── ANSI helpers ────────────────────────────────────────────────────────────

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

// ── Header ──────────────────────────────────────────────────────────────────

const HEADER: &str = "\
\u{2580}\u{2580}\u{2588}\u{2580}\u{2580} \u{2592}\u{2588}\u{2580}\u{2580}\u{2580} \u{2592}\u{2588}\u{2580}\u{2580}\u{2588} \u{2592}\u{2588}\u{2580}\u{2584}\u{2580}\u{2588} \u{2591}\u{2580}\u{2591} \u{2592}\u{2588}\u{2584}\u{2591}\u{2592}\u{2588} \u{2591}\u{2588}\u{2580}\u{2580}\u{2588} \u{2592}\u{2588}\u{2591}\u{2591}\u{2591}
\u{2591}\u{2592}\u{2588}\u{2591}\u{2591} \u{2592}\u{2588}\u{2580}\u{2580}\u{2580} \u{2592}\u{2588}\u{2584}\u{2584}\u{2580} \u{2592}\u{2588}\u{2592}\u{2588}\u{2592}\u{2588} \u{2580}\u{2588}\u{2580} \u{2592}\u{2588}\u{2592}\u{2588}\u{2592}\u{2588} \u{2592}\u{2588}\u{2584}\u{2584}\u{2588} \u{2592}\u{2588}\u{2591}\u{2591}\u{2591}
\u{2591}\u{2592}\u{2588}\u{2591}\u{2591} \u{2592}\u{2588}\u{2584}\u{2584}\u{2584} \u{2592}\u{2588}\u{2591}\u{2592}\u{2588} \u{2592}\u{2588}\u{2591}\u{2591}\u{2592}\u{2588} \u{2580}\u{2580}\u{2580} \u{2592}\u{2588}\u{2591}\u{2591}\u{2580}\u{2588} \u{2592}\u{2588}\u{2591}\u{2592}\u{2588} \u{2592}\u{2588}\u{2584}\u{2584}\u{2588}";

pub fn print_header(rarity: &Rarity) {
    let color = rarity_color(rarity);
    println!();
    for line in HEADER.lines() {
        println!("  {}{}{}", color, line, RESET);
    }
    println!();
}

// ── Pet display ─────────────────────────────────────────────────────────────

pub fn print_drop(drop: &PetDrop, state: &State, is_new_drop: bool) {
    let rarity = Rarity::from_str(&drop.rarity);
    let color = rarity_color(&rarity);

    let pet = match find_pet_by_id(&drop.species) {
        Some(p) => p,
        None => return,
    };
    let art = render_art(pet, &drop.eye, hat_line_for(&drop.hat));
    let art_width = art.iter().map(|l| visual_len(l)).max().unwrap_or(0);

    let species_count = state.species_count(&drop.species);

    let drop_line = if is_new_drop {
        format!("{}{}  ✦ New drop! ✦{}", BOLD, color, RESET)
    } else {
        String::new()
    };
    let name_line = format!("{}{}  {}{}", BOLD, color, drop.pet_name, RESET);
    let rarity_line = format!(
        "{}{}  {} {}{}",
        BOLD, color, rarity.stars(), drop.rarity, RESET
    );
    let count_line = if species_count > 1 {
        format!(
            "{}  {} of {} {}s{}",
            DIM, drop_number(drop, state), species_count, pet.name, RESET
        )
    } else {
        String::new()
    };

    let info: Vec<&str> = [&drop_line, &name_line, &rarity_line, &count_line]
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .collect();

    let total_lines = art.len().max(info.len());
    for i in 0..total_lines {
        let art_line = art.get(i).map(|s| s.as_str()).unwrap_or("");
        let info_line = info.get(i).copied().unwrap_or("");
        println!(
            "{}{}{:<pad$}{}{}",
            color, art_line, "", RESET, info_line,
            pad = art_width.saturating_sub(visual_len(art_line)),
        );
    }
    println!();
}

/// Which number (1-based) this drop is among drops of the same species.
fn drop_number(drop: &PetDrop, state: &State) -> usize {
    state.drops.iter()
        .filter(|d| d.species == drop.species)
        .position(|d| d.id == drop.id)
        .map(|i| i + 1)
        .unwrap_or(1)
}

// ── Collection view ─────────────────────────────────────────────────────────

pub fn print_collection(state: &State) {
    if state.drops.is_empty() {
        println!("  No pets collected yet. Open a terminal to get your first drop!");
        println!();
        return;
    }

    let default_id = state.default_drop_id.as_deref().unwrap_or("");

    println!("  {:<3} {:<22} {:<12} {:<10} {:<4} {}",
        "#", "Name", "Species", "Rarity", "Eye", "Hat");
    println!("  {}", "─".repeat(66));

    for (i, drop) in state.drops.iter().enumerate() {
        let rarity = Rarity::from_str(&drop.rarity);
        let color = rarity_color(&rarity);
        let default_marker = if drop.id == default_id {
            format!("  {}[default]{}", "\x1b[36m", RESET)
        } else {
            String::new()
        };
        let hat_display = if drop.hat == "none" { "—" } else { drop.hat.as_str() };
        let num   = format!("{}{}{}", DIM, i + 1, RESET);
        let rstr  = format!("{}{} {}{}", color, rarity.stars(), drop.rarity, RESET);
        println!(
            "  {:<12} {:<22} {:<12} {:<24} {:<4} {}{}",
            num, drop.pet_name, drop.species, rstr, drop.eye, hat_display, default_marker,
        );
    }
    println!();
    println!("  {}Use `greeting set <number>` to set a default.{}", DIM, RESET);
    println!();
}

// ── Catalog view ─────────────────────────────────────────────────────────────

pub fn print_catalog(state: &State) {
    use crate::pets::catalog;

    let total = catalog().len();
    let discovered = catalog().iter().filter(|p| state.owns_species(p.id)).count();
    println!("  {}{}/{} species discovered{}", DIM, discovered, total, RESET);
    println!();

    for pet in catalog() {
        let count = state.species_count(pet.id);
        if count > 0 {
            println!("  {:<12}  {} drop{}", pet.name, count, if count == 1 { "" } else { "s" });
        } else {
            println!("  {}???{}", DIM, RESET);
        }
    }
    println!();
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn visual_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' { in_escape = true; }
        else if in_escape { if c == 'm' { in_escape = false; } }
        else { len += 1; }
    }
    len
}
