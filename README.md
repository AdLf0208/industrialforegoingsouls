# Industrial Foregoing: Souls — a Horizon port

A port of [Industrial Foregoing: Souls](https://github.com/InnovativeOnlineIndustries/Industrial-Foregoing-Souls)
for **Horizon** (Minecraft: Java Edition 26.3 written in Rust). MIT, as upstream.

The mod is one soul economy in three blocks:

- **Soul Laser Base** — seats an `industrialforegoing:blue_laser_lens` in its catalyst slot and, with
  a warden in the 3×3×3 box above it, hurts the warden for 4 every 20 ticks and stores 2 souls in a
  1,350-soul tank. An `industrialforegoing:efficiency_addon_*` seats in its augment slot and rescales
  the bar to `floor(20 × efficiency)` after an operation (the tier 2 addon makes it 16 ticks); a
  speed addon is refused.
- **Soul Pipe** — carries souls: 4 a block of capacity, 4 a tick pulled out of an adjacent soul
  handler and pushed into one that accepts.
- **Soul Surge** — spends one soul on 300 accelerated ticks and random-ticks the block it is mounted
  on four times at a 3% chance a tick (a crop grows, where `random_tick_speed 0` means nothing else
  does), and stops while it is powered.

## Install

This mod **requires Industrial Foregoing** (its blue laser lens is the laser base's gate). Install
both; a game without the parent refuses this package at load with
`mod industrialforegoingsouls requested unknown item industrialforegoing:blue_laser_lens`.

Horizon mods are single `.hmod` files (code and content together), built by the Horizon launcher
against the pinned game build: `industrialforegoingsouls.hmod` is the package.

In play: place the laser base, right-click it with a blue lens to seat it (an empty hand takes it
back), right-click with an efficiency addon to seat that too, put a warden above it, and run the
souls into a pipe network with a surge on whatever you want to accelerate.
`/industrialforegoingsouls info <x> <y> <z>` prints a block's whole state; `souls` and `wait`
(operator) set it.

The package also carries the mod's one `industrialforegoing:laser_drill_ore` recipe — an echo shard
from the deep dark — so the parent's ore laser base can mine it.

## What this port does not do

- The comparator output (the formula is implemented and unit-tested; the `analog_output` hook cannot
  read a block entity at this pin).
- The surge's machine-accelerating and entity-accelerating branches (no helper ticks another block
  entity, or an entity).
- The Sculk Soul Tank screen addon (the parent GUI it draws on is not ported).
- A screen for the two augment slots: the click is the only way in or out, and a broken machine does
  not carry its augments or its lens on its item.
- A config file: the numbers are constants.
- Waterlogging, and the surge's `enabled`/connection block states (the redstone rule is read in the
  tick instead).

`notes.md` records every deviation with its reason, the coverage table, and the in-game evidence;
`FINDINGS.md` is the upstream behaviour; `VALIDATION.md` is the ladder that was run.

## Building and testing

```
cargo test                                                  # the pure runtime, 22 tests
cargo fmt --check && cargo clippy --all-targets -- -D warnings
rm -f Cargo.lock && cargo horizon-mod build                 # -> industrialforegoingsouls.hmod
cargo horizon-mod check industrialforegoingsouls.hmod
cargo horizon-mod fits industrialforegoingsouls.hmod --build 0b14f8a
python3 tools/assemble_content.py --check                    # assets/ and data/ match the jar carry
tools/e2e-two-mod.sh run                                     # builds this port + its parent, runs tools/e2e.sh
```

`tools/e2e-two-mod.sh` exists because this port needs its parent in the same game; see
`VALIDATION.md` "Notes on the run".
