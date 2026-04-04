use rand::Rng;

use crate::pets::{catalog, random_name, Pet, Rarity, ALL_RARITIES, EYES, HATS};
use crate::state::PetDrop;

pub fn roll_rarity() -> Rarity {
    let total: u32 = ALL_RARITIES.iter().map(|r| r.weight()).sum();
    let mut roll = rand::thread_rng().gen_range(0..total);
    for rarity in ALL_RARITIES {
        if roll < rarity.weight() {
            return rarity.clone();
        }
        roll -= rarity.weight();
    }
    Rarity::Common
}

pub fn roll_species() -> &'static Pet {
    let pets = catalog();
    &pets[rand::thread_rng().gen_range(0..pets.len())]
}

fn roll_eye() -> &'static str {
    EYES[rand::thread_rng().gen_range(0..EYES.len())]
}

fn roll_hat() -> (&'static str, &'static str) {
    HATS[rand::thread_rng().gen_range(0..HATS.len())]
}

fn gen_id() -> String {
    let chars: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..4).map(|_| chars[rng.gen_range(0..chars.len())] as char).collect()
}

/// Roll a complete pet drop — everything is determined here and never changes.
pub fn roll_drop() -> PetDrop {
    let rarity = roll_rarity();
    let species = roll_species();
    let eye = roll_eye();
    let (hat_name, _) = roll_hat();
    let pet_name = random_name(species.name);

    PetDrop {
        id: gen_id(),
        species: species.id.to_string(),
        rarity: rarity.as_str().to_string(),
        eye: eye.to_string(),
        hat: hat_name.to_string(),
        pet_name,
    }
}

