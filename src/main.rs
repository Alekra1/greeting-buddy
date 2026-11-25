mod collection;
mod display;
mod font_atlas;
mod gfx;
mod pets;
mod scenes;
mod scenes_hd;
mod state;
mod sysinfo;

use clap::{Parser, Subcommand};
use collection::roll_drop;
use display::{print_catalog, print_collection, print_greeting_box, rarity_color};
use pets::Rarity;
use scenes::Scene;
use state::State;
use sysinfo::SysInfo;

#[derive(Parser)]
#[command(
    name = "greeting",
    about = "TrueColor dark-fantasy ASCII terminal greeting with live random scene rotation"
)]
struct Cli {
    /// Start on a specific scene: eclipse, bonfire, torii, portal, grimoire, nether (or 1-6)
    #[arg(short, long)]
    scene: Option<String>,

    /// Seconds between automatic random scene switches (0 disables auto-switch)
    #[arg(short, long, default_value_t = 5)]
    interval: u64,

    /// Render a single static frame immediately and return to the shell
    #[arg(long = "static")]
    static_mode: bool,

    /// Run as a foreground live viewer (press q/Enter/Esc to exit)
    #[arg(long = "fg")]
    foreground: bool,

    /// Force pure-text 24-bit TrueColor rendering (disable Kitty Graphics HD bloom)
    #[arg(long = "text")]
    text_mode: bool,

    /// Internal: dump raw 462x330 RGBA8888 bytes of one HD bloom frame to a file
    #[arg(long = "dump-rgba", hide = true)]
    dump_rgba: Option<String>,

    /// Internal: animation timestamp (seconds) for --dump-rgba
    #[arg(long = "dump-time", default_value_t = 2.4, hide = true)]
    dump_time: f32,

    /// Internal: run the background top-banner animator
    #[arg(long = "daemon", hide = true)]
    daemon: bool,

    /// Internal: parent shell PID for daemon lifecycle tracking
    #[arg(long = "shell-pid", hide = true)]
    shell_pid: Option<i32>,

    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show the classic starry pet greeting card
    Pet,
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

fn show_classic_pet(si: &SysInfo) {
    let mut state = State::load();
    if let Some(drop) = state.default_drop().cloned() {
        print_greeting_box(&drop, &state, false, si);
    } else if state.can_drop() {
        let drop = roll_drop();
        state.add_drop(drop.clone());
        state.save();
        print_greeting_box(&drop, &state, true, si);
    } else if let Some(drop) = state.drops.last().cloned() {
        print_greeting_box(&drop, &state, false, si);
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        // ── Default: TrueColor live ASCII greeting (random scene every 5s) ───
        None => {
            let initial = match cli.scene.as_deref() {
                None | Some("random") => None,
                Some(name) => match Scene::from_name(name) {
                    Some(s) => Some(s),
                    None => {
                        eprintln!(
                            "Unknown scene '{}'. Valid scenes: eclipse, bonfire, torii, portal, grimoire, nether (or 1-6).",
                            name
                        );
                        std::process::exit(1);
                    }
                },
            };

            if let Some(ref dump_path) = cli.dump_rgba {
                let target = initial.unwrap_or(Scene::Eclipse);
                if let Err(e) = scenes::dump_hd_rgba(target, cli.dump_time, dump_path) {
                    eprintln!("Failed to dump RGBA: {}", e);
                    std::process::exit(1);
                }
                return;
            }

            let si = SysInfo::gather();
            if cli.daemon {
                let spid = cli.shell_pid.unwrap_or_else(|| unsafe { libc::getppid() });
                scenes::run_daemon(initial, cli.interval, spid, cli.text_mode, &si);
            } else if cli.static_mode {
                scenes::print_static(initial, cli.text_mode, &si);
            } else if cli.foreground {
                scenes::run_foreground(initial, cli.interval, cli.text_mode, &si);
            } else {
                scenes::start_background_banner(initial, cli.interval, cli.text_mode, &si);
            }
        }

        // ── Classic pet card ─────────────────────────────────────────────────
        Some(Cmd::Pet) => {
            let si = SysInfo::gather();
            show_classic_pet(&si);
        }

        // ── Roll: force a new drop ───────────────────────────────────────────
        Some(Cmd::Roll) => {
            let mut state = State::load();
            let wait = state.secs_until_drop();
            if wait > 0 {
                let mins = wait / 60;
                let secs = wait % 60;
                if mins > 0 {
                    println!("Next drop available in {}m {}s.", mins, secs);
                } else {
                    println!("Next drop available in {}s.", secs);
                }
                return;
            }
            let drop = roll_drop();
            state.add_drop(drop.clone());
            state.save();
            let si = SysInfo::gather();
            print_greeting_box(&drop, &state, true, &si);
        }

        // ── Collection ───────────────────────────────────────────────────────
        Some(Cmd::Collection) => {
            let state = State::load();
            println!();
            println!(
                "  \x1b[1mYour Collection\x1b[0m  \x1b[2m({} drops)\x1b[0m",
                state.drops.len()
            );
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
                    number,
                    state.drops.len()
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
