// ── Name generation ─────────────────────────────────────────────────────────

pub const ADJECTIVES: &[&str] = &[
    "Ancient", "Azure", "Brave", "Bold", "Chunky", "Cosmic", "Crimson",
    "Curious", "Daring", "Dreamy", "Eternal", "Fierce", "Frosty", "Fluffy",
    "Gentle", "Gloomy", "Golden", "Grumpy", "Hollow", "Humble", "Jolly",
    "Lanky", "Mighty", "Mystic", "Noble", "Peculiar", "Phantom", "Radiant",
    "Rusty", "Sacred", "Serene", "Shadow", "Silver", "Sleepy", "Spooky",
    "Stoic", "Swift", "Tiny", "Valiant", "Verdant", "Virtuous", "Vivid",
    "Whimsical", "Wise", "Wobbly",
];

pub fn random_name(species_name: &str) -> String {
    let idx = rand::random::<usize>() % ADJECTIVES.len();
    format!("{} {}", ADJECTIVES[idx], species_name)
}

// ── Eyes (6 options, equal probability) ─────────────────────────────────────

pub const EYES: &[&str] = &["·", "✦", "×", "◉", "@", "°"];

#[derive(Debug, Clone, PartialEq)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
            Rarity::Legendary => "Legendary",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Uncommon" => Rarity::Uncommon,
            "Rare" => Rarity::Rare,
            "Epic" => Rarity::Epic,
            "Legendary" => Rarity::Legendary,
            _ => Rarity::Common,
        }
    }

    pub fn stars(&self) -> &'static str {
        match self {
            Rarity::Common => "★",
            Rarity::Uncommon => "★★",
            Rarity::Rare => "★★★",
            Rarity::Epic => "★★★★",
            Rarity::Legendary => "★★★★★",
        }
    }

    /// Weights from types.ts: 60 / 25 / 10 / 4 / 1
    pub fn weight(&self) -> u32 {
        match self {
            Rarity::Common => 60,
            Rarity::Uncommon => 25,
            Rarity::Rare => 10,
            Rarity::Epic => 4,
            Rarity::Legendary => 1,
        }
    }
}

pub const ALL_RARITIES: &[Rarity] = &[
    Rarity::Common,
    Rarity::Uncommon,
    Rarity::Rare,
    Rarity::Epic,
    Rarity::Legendary,
];

#[derive(Debug, Clone)]
pub struct Pet {
    pub id: &'static str,
    pub name: &'static str,
    /// Frame-0 art from sprites.ts. Line 0 is blank vertical spacing.
    /// Uses {E} as the eye placeholder.
    pub art: &'static [&'static str],
}

/// Render art: substitute eye and drop the blank top spacing line.
pub fn render_art(pet: &Pet, eye: &str) -> Vec<String> {
    let mut lines: Vec<String> = pet.art.iter()
        .map(|l| l.replace("{E}", eye))
        .collect();
    lines.remove(0);
    lines
}

pub fn catalog() -> &'static [Pet] {
    &PETS
}

pub fn find_pet_by_id(id: &str) -> Option<&'static Pet> {
    PETS.iter().find(|p| p.id == id)
}

// Exact frame-0 sprites from sprites.ts, all 18 species.
// Line 0 is always blank in frame 0.
static PETS: &[Pet] = &[
    Pet {
        id: "duck",
        name: "Duck",
        art: &[
            "            ",
            "    __      ",
            "  <({E} )___  ",
            "   (  ._>   ",
            "    `--´    ",
        ],
    },
    Pet {
        id: "goose",
        name: "Goose",
        art: &[
            "            ",
            "     ({E}>    ",
            "     ||     ",
            "   _(__)_   ",
            "    ^^^^    ",
        ],
    },
    Pet {
        id: "blob",
        name: "Blob",
        art: &[
            "            ",
            "   .----.   ",
            "  ( {E}  {E} )  ",
            "  (      )  ",
            "   `----´   ",
        ],
    },
    Pet {
        id: "cat",
        name: "Cat",
        art: &[
            "            ",
            "   /\\_/\\    ",
            "  ( {E}   {E})  ",
            "  (  ω  )   ",
            "  (\")_(\")   ",
        ],
    },
    Pet {
        id: "dragon",
        name: "Dragon",
        art: &[
            "            ",
            "  /^\\  /^\\  ",
            " <  {E}  {E}  > ",
            " (   ~~   ) ",
            "  `-vvvv-´  ",
        ],
    },
    Pet {
        id: "octopus",
        name: "Octopus",
        art: &[
            "            ",
            "   .----.   ",
            "  ( {E}  {E} )  ",
            "  (______)  ",
            "  /\\/\\/\\/\\  ",
        ],
    },
    Pet {
        id: "owl",
        name: "Owl",
        art: &[
            "            ",
            "   /\\  /\\   ",
            "  (({E})({E}))  ",
            "  (  ><  )  ",
            "   `----´   ",
        ],
    },
    Pet {
        id: "penguin",
        name: "Penguin",
        art: &[
            "            ",
            "  .---.     ",
            "  ({E}>{E})     ",
            " /(   )\\    ",
            "  `---´     ",
        ],
    },
    Pet {
        id: "turtle",
        name: "Turtle",
        art: &[
            "            ",
            "   _,--._   ",
            "  ( {E}  {E} )  ",
            " /[______]\\ ",
            "  ``    ``  ",
        ],
    },
    Pet {
        id: "snail",
        name: "Snail",
        art: &[
            "            ",
            " {E}    .--.  ",
            "  \\  ( @ )  ",
            "   \\_`--´   ",
            "  ~~~~~~~   ",
        ],
    },
    Pet {
        id: "ghost",
        name: "Ghost",
        art: &[
            "            ",
            "   .----.   ",
            "  / {E}  {E} \\  ",
            "  |      |  ",
            "  ~`~``~`~  ",
        ],
    },
    Pet {
        id: "axolotl",
        name: "Axolotl",
        art: &[
            "            ",
            "}~(______)~{",
            "}~({E} .. {E})~{",
            "  ( .--. )  ",
            "  (_/  \\_)  ",
        ],
    },
    Pet {
        id: "capybara",
        name: "Capybara",
        art: &[
            "            ",
            "  n______n  ",
            " ( {E}    {E} ) ",
            " (   oo   ) ",
            "  `------´  ",
        ],
    },
    Pet {
        id: "cactus",
        name: "Cactus",
        art: &[
            "            ",
            " n  ____  n ",
            " | |{E}  {E}| | ",
            " |_|    |_| ",
            "   |    |   ",
        ],
    },
    Pet {
        id: "robot",
        name: "Robot",
        art: &[
            "            ",
            "   .[||].   ",
            "  [ {E}  {E} ]  ",
            "  [ ==== ]  ",
            "  `------´  ",
        ],
    },
    Pet {
        id: "rabbit",
        name: "Rabbit",
        art: &[
            "            ",
            "   (\\__/)   ",
            "  ( {E}  {E} )  ",
            " =(  ..  )= ",
            "  (\")__(\")  ",
        ],
    },
    Pet {
        id: "mushroom",
        name: "Mushroom",
        art: &[
            "            ",
            " .-o-OO-o-. ",
            "(__________)",
            "   |{E}  {E}|   ",
            "   |____|   ",
        ],
    },
    Pet {
        id: "chonk",
        name: "Chonk",
        art: &[
            "            ",
            "  /\\    /\\  ",
            " ( {E}    {E} ) ",
            " (   ..   ) ",
            "  `------´  ",
        ],
    },
];
