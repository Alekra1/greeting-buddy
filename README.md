# greeting

A terminal greeting with a collectible pet system. Every time you open a terminal you get a random pet drop — species, rarity, eye style, and a generated name are all rolled and **permanently saved**. Collect them all.

```
  ▀▀█▀▀ ▒█▀▀▀ ▒█▀▀█ ▒█▀▄▀█ ░▀░ ▒█▄░▒█ ░█▀▀█ ▒█░░░
  ░▒█░░ ▒█▀▀▀ ▒█▄▄▀ ▒█▒█▒█ ▀█▀ ▒█▒█▒█ ▒█▄▄█ ▒█░░░
  ░▒█░░ ▒█▄▄▄ ▒█░▒█ ▒█░░▒█ ▀▀▀ ▒█░░▀█ ▒█░▒█ ▒█▄▄█

   /^\
  .----.       Virtuous Ghost
 / °  ° \      ★★★★ Epic
 |      |
 ~`~``~`~
```

---

## Install

```bash
git clone <repo> && cd greeting
cargo install --path .
```

Make sure `~/.cargo/bin` is on your PATH:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
```

## Run on terminal open

Add a single line to your `~/.bashrc` (or `~/.zshrc`):

```bash
echo 'greeting' >> ~/.bashrc
```

> **Important:** use `echo '...' >>` exactly once — running it multiple times duplicates the line.
> Verify with `grep -c 'greeting' ~/.bashrc` — should return `1`.

---

## Drop system

Each terminal open (or `greeting roll`) independently rolls:

| Roll | Options | Probability |
|------|---------|-------------|
| **Rarity** | Common / Uncommon / Rare / Epic / Legendary | 60% / 25% / 10% / 4% / 1% |
| **Species** | 18 species | equal |
| **Eye** | `·` `✦` `×` `◉` `@` `°` | equal |
| **Name** | adjective + species (e.g. *Virtuous Ghost*, *Frosty Duck*) | — |

All four values are locked in at drop time and saved permanently. The same species can drop multiple times as different pets with different rarities, eyes, and names.

The header color, pet art, rarity label, and species count all use the rarity color of the displayed drop.

---

## CLI

```
greeting                    Open greeting — rolls a new drop (or shows default)
greeting roll               Force a new drop regardless of default
greeting collection         List all your drops with numbers, names, rarities
greeting set <number>       Set default pet by number from `greeting collection`
greeting unset              Clear default — go back to random drops
greeting catalog            Show all 18 species and how many of each you own
```

### Workflow example

```bash
# Roll some pets
greeting roll
greeting roll

# See your full collection with numbers
greeting collection
#  #   Name                   Species      Rarity          Eye
#  1   Virtuous Ghost         ghost        ★★★★ Epic       °
#  2   Frosty Duck            duck         ★ Common        ·
#  3   Bold Ghost             ghost        ★ Common        ✦

# Keep the epic ghost as your default
greeting set 1

# Verify
greeting
# → shows Virtuous Ghost every time, same rarity/eye

# Go back to random drops
greeting unset
```

### Species

`duck` `goose` `blob` `cat` `dragon` `octopus` `owl` `penguin` `turtle`
`snail` `ghost` `axolotl` `capybara` `cactus` `robot` `rabbit` `mushroom` `chonk`

---

## Config files

| File | Purpose |
|------|---------|
| `~/.config/terminal-greeting/state.json` | All your drops + which one is the default |

The state file is created automatically on first run. Delete it to reset your collection.

```json
{
  "default_drop_id": "a3f9",
  "drops": [
    {
      "id": "a3f9",
      "species": "ghost",
      "rarity": "Epic",
      "eye": "°",
      "pet_name": "Virtuous Ghost"
    },
    {
      "id": "b1k2",
      "species": "duck",
      "rarity": "Common",
      "eye": "·",
      "pet_name": "Frosty Duck"
    }
  ]
}
```
