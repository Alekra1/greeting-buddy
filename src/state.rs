use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const DROP_COOLDOWN_SECS: u64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetDrop {
    pub id: String,
    pub species: String,   // "ghost"
    pub rarity: String,    // "Epic"
    pub eye: String,       // "°"
    pub hat: String,       // "crown" / "none"
    pub pet_name: String,  // "Virtuous Ghost"
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct State {
    pub default_drop_id: Option<String>,
    pub drops: Vec<PetDrop>,
    pub last_drop_time: Option<u64>,
}

impl State {
    pub fn load() -> Self {
        match fs::read_to_string(state_path()) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = state_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).expect("serialization failed");
        fs::write(&path, json).expect("failed to write state file");
    }

    pub fn add_drop(&mut self, drop: PetDrop) {
        self.drops.push(drop);
        self.last_drop_time = Some(now_secs());
    }

    /// Seconds remaining until next drop is allowed, or 0 if ready.
    pub fn secs_until_drop(&self) -> u64 {
        match self.last_drop_time {
            None => 0,
            Some(t) => {
                let elapsed = now_secs().saturating_sub(t);
                DROP_COOLDOWN_SECS.saturating_sub(elapsed)
            }
        }
    }

    pub fn can_drop(&self) -> bool {
        self.secs_until_drop() == 0
    }

    pub fn default_drop(&self) -> Option<&PetDrop> {
        let id = self.default_drop_id.as_deref()?;
        self.drops.iter().find(|d| d.id == id)
    }

    /// Count how many times a species has been collected.
    pub fn species_count(&self, species: &str) -> usize {
        self.drops.iter().filter(|d| d.species == species).count()
    }

    /// True if the player owns at least one drop of this species.
    pub fn owns_species(&self, species: &str) -> bool {
        self.drops.iter().any(|d| d.species == species)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn state_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("terminal-greeting")
        .join("state.json")
}
