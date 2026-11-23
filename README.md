# Industrial Foregoing: Souls — a Horizon port

A port of [Industrial Foregoing: Souls](https://github.com/InnovativeOnlineIndustries/Industrial-Foregoing-Souls)
for **Horizon** (Minecraft: Java Edition 26.3 written in Rust). MIT, as upstream.

The mod is one soul economy in three blocks:

- **Soul Laser Base** — seats an `industrialforegoing:blue_laser_lens` in its catalyst slot and, with
  a warden in the 3×3×3 box above it, hurts the warden for 4 every 20 ticks and stores 2 souls in a
  1,350-soul tank. An `industrialforegoing:efficiency_addon_*` seats in its augment slot and rescales
  the bar to `floor(20 × efficiency)` after an operation (the tier 2 addon makes it 16 ticks); a
  speed addon is refused. Its **menu** (a plain right-click) shows the catalyst slot, the two augment
  slots and the tank and the bar, as upstream's screen does.
- **Soul Pipe** — carries souls: 4 a block of capacity, 4 a tick pulled out of an adjacent soul
  handler and pushed into one that accepts.
- **Soul Surge** — spends one soul on 300 accelerated ticks and random-ticks the block it is mounted
  on four times at a 3% chance a tick (a crop grows, where `random_tick_speed 0` means nothing else
  does), and stops while it is powered.

## Install

This mod **requires Industrial Foregoing** (its blue laser lens is the laser base's gate, and the
carried ore recipe is the parent's type). `horizon-mod.toml` declares the dependency
(`industrialforegoing >= 0.2.7`); a bundle without the parent is refused by name with
`industrialforegoingsouls requires missing mod industrialforegoing` before it starts, instead of
failing later at registration.

Horizon mods are single `.hmod` files (code and content together), built by the Horizon launcher
against the pinned game build: `industrialforegoingsouls.hmod` is the package.

In play: place the laser base, right-click it with a blue lens to seat it (or open its menu and put
it in the catalyst slot there), right-click with an efficiency addon to seat that too, put a warden
above it, and run the souls into a pipe network with a surge on whatever you want to accelerate. The
menu's two augment slots and the catalyst are how the parts come back out.
`/industrialforegoingsouls info <x> <y> <z>` prints a block's whole state; `souls` and `wait`
(operator) set it.

## Config

Every number is the jar's own config setting, and `config/industrialforegoingsouls/config.json`
overrides it. The game writes the file with the jar's defaults when it is missing:

```json
{
  "soul_storage": 1350,
  "max_progress": 20,
  "kill_warden": false,
  "damage_per_operation": 4,
  "souls_per_operation": 2,
  "soul_time": 300,
  "acceleration_tick": 4,
  "entities_acceleration_tick": 4,
  "block_acceleration_tick": 4,
  "random_tick_acceleration_chance": 0.03,
  "soul_per_pipe": 4,
  "pipes_particles": 0.1
}
```

The tank, the bar, the damage and the souls of an operation, whether a warden may be killed, a
soul's ticks, the surge's acceleration ticks and chance, and a pipe's share of a network. The file
is read once when the game loads it and again when it changes on disk — never from a tick — and a
value outside the jar's own range is clamped (`soul_storage`/`max_progress`/`soul_time`/
`souls_per_operation` at least 1, the two chances between 0 and 1). A damaged or partial file keeps
the defaults for the keys it does not name; it cannot crash the game. `acceleration_tick`,
`entities_acceleration_tick` and `pipes_particles` are accepted and kept for the two acceleration
branches and the client half this port does not ship (below).

The package also carries the mod's one `industrialforegoing:laser_drill_ore` recipe — an echo shard
from the deep dark — so the parent's ore laser base can mine it.

The pipe and the surge also **hold water**, as upstream's `SimpleWaterloggedBlock` does: right-click
either with a water bucket to fill it and with an empty bucket to take the water back; the machine
keeps working either way. The full bucket a drain takes goes to the first free inventory slot (this
pin reverts a water bucket written into the held slot from a click — `notes.md` §1.4).

## What this port does not do

- The comparator output (the formula is implemented and unit-tested; the `analog_output` hook cannot
  read a block entity at this pin).
- The surge's machine-accelerating and entity-accelerating branches (no helper ticks another block
  entity, or an entity).
- The Sculk Soul Tank screen addon (the parent GUI it draws on is not ported).
- A **texture** for the machine's screen: the parent's Titanium screen (and its GUI texture) is not
  this package's and the jar carries none, so the tank and the bar print over the engine's plain
  panel; the screen's labels are literal English. A broken machine carries its contents on its item
  (souls, the bar, the lens, the addon), and placing it back restores them; a second lens swaps with
  the seated one, which comes back to the player.
- The waterloggable **interface** itself: a flowing fluid beside the pipe does not fill it and a
  dispenser does not either (a content block's interfaces are its template's), and the port does not
  invent a placement-into-water override the findings do not record. The state, the water the engine
  reports for it and the player's bucket are ported.
- The surge's `enabled`/connection block states (the redstone rule is read in the tick instead).

`notes.md` records every deviation with its reason, the coverage table, and the in-game evidence;
`FINDINGS.md` is the upstream behaviour; `VALIDATION.md` is the ladder that was run.

## Building and testing

```
cargo test                                                  # the pure runtime and its rules, 60 tests
cargo fmt --check && cargo clippy --all-targets -- -D warnings
rm -f Cargo.lock && cargo horizon-mod build                 # -> industrialforegoingsouls.hmod
cargo horizon-mod check industrialforegoingsouls.hmod
cargo horizon-mod fits industrialforegoingsouls.hmod --build 0b14f8a
python3 tools/assemble_content.py --check                    # assets/ and data/ match the jar carry
tools/e2e-two-mod.sh run                                     # builds this port + its parent, runs tools/e2e.sh
```

`tools/e2e-two-mod.sh` exists because this port needs its parent in the same game; see
`VALIDATION.md` "Notes on the run".
