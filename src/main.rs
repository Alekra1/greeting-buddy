mod collection;
mod display;
mod pets;
mod state;

use clap::{Parser, Subcommand};
use collection::roll_drop;
use display::{print_catalog, print_collection, print_drop, print_header, rarity_color};
use pets::Rarity;
use state::State;

#[derive(Parser)]
#[command(name = "greeting", about = "Terminal greeting with collectible pets")]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Force a new random pet drop
    Roll,
    /// List all your collected drops
    Collection,
    /// Set default pet by number (from `greeting collection`)
    Set {
        /// Drop number shown in `greeting collection`
        number: usize,
    },
    /// Clear default pet (go back to random drops)
    Unset,
    /// Show all 18 species and how many you've collected
    Catalog,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        // ── Default: show greeting ───────────────────────────────────────────
        None => {
            let mut state = State::load();

            if let Some(drop) = state.default_drop().cloned() {
                let rarity = Rarity::from_str(&drop.rarity);
                print_header(&rarity);
                print_drop(&drop, &state, false);
            } else {
                let drop = roll_drop();
                let rarity = Rarity::from_str(&drop.rarity);
                print_header(&rarity);
                state.add_drop(drop.clone());
                state.save();
                print_drop(&drop, &state, true);
            }
        }

        // ── Roll: force a new drop ───────────────────────────────────────────
        Some(Cmd::Roll) => {
            let mut state = State::load();
            let drop = roll_drop();
            let rarity = Rarity::from_str(&drop.rarity);
            print_header(&rarity);
            state.add_drop(drop.clone());
            state.save();
            print_drop(&drop, &state, true);
        }

        // ── Collection ───────────────────────────────────────────────────────
        Some(Cmd::Collection) => {
            let state = State::load();
            println!();
            println!("  \x1b[1mYour Collection\x1b[0m  \x1b[2m({} drops)\x1b[0m",
                state.drops.len());
            println!();
            print_collection(&state);
        }

        // ── Set default by 1-based index ─────────────────────────────────────
        Some(Cmd::Set { number }) => {
            let mut state = State::load();
            if state.drops.is_empty() {
                eprintln!("No pets collected yet.");
                return;
            }
            if number == 0 || number > state.drops.len() {
                eprintln!(
                    "Invalid number {}. You have {} drops (use `greeting collection` to see them).",
                    number, state.drops.len()
                );
                return;
            }
            let drop = state.drops[number - 1].clone();
            let color = rarity_color(&Rarity::from_str(&drop.rarity));
            let name = drop.pet_name.clone();
            state.default_drop_id = Some(drop.id);
            state.save();
            println!("Default pet set to {}{}\x1b[0m.", color, name);
        }

        // ── Unset ────────────────────────────────────────────────────────────
        Some(Cmd::Unset) => {
            let mut state = State::load();
            state.default_drop_id = None;
            state.save();
            println!("Default pet cleared. You'll get random drops again.");
        }

        // ── Catalog ──────────────────────────────────────────────────────────
        Some(Cmd::Catalog) => {
            let state = State::load();
            println!();
            println!("  \x1b[1mPet Catalog\x1b[0m");
            println!();
            print_catalog(&state);
        }
    }
}
