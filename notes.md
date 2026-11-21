# `industrialforegoingsouls` — Industrial Foregoing: Souls 0.2.0

The three blocks that are one soul economy, ported from **Industrial Foregoing: Souls 1.21.1-1.10.7**
into Rust for Horizon 26.3 at the campaign pin.

| | |
|---|---|
| class · tier | **content · machines** (`ports/ledger.csv`) |
| licence | **MIT**, read from the jar's `META-INF/neoforge.mods.toml` (`audit_licences.py --mod industrialforegoingsouls --fetch` → `declared MIT / effective MIT`), rank **0** |
| approach (`horizon/docs/modpacks.md`) | **agent port** of the behaviour, with the jar's content carried (models, blockstates, textures, recipes, loot, tags) |
| pin | Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`, horizon-mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| upstream | `InnovativeOnlineIndustries/Industrial-Foregoing-Souls`, commit `bf736d2f1140da25747ab97d8c160b44d8a0bb7f` (2026-01-10, the jar's `1.10.7`); the jar is `ports/_shared/licence-cache/industrial-foregoing-souls-1.21.1-1.10.7.jar`, sha256 `f4fb3640…67ec` |
| package | `industrialforegoingsouls.hmod`, **77,206 B**, sha256 `9169cbc45d0e60c2fdb74f2d75fcbdf541b1f9b94a4509a0b0566f1495f91533`, 59 entries, no packed `Cargo.lock` |
| registry | not on `main` → a **first publication** (`Add industrialforegoingsouls 0.2.0`) |

Skills: superpowers/using-superpowers,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices

## 1. The slice

The mod is **one mechanic in three blocks**, and a package with only one of them is inert, so the
slice is all three:

- **`soul_laser_base`** — a block entity with a 20-tick bar and one catalyst slot. With
  `industrialforegoing:blue_laser_lens` seated and a warden inside `Shapes.box(-1,0,-1,2,3,2)` over
  the machine, every 20 ticks it hurts the warden for 4 and stores 2 souls, up to 1,350.
- **`soul_network_pipe`** — a network element holding 4 souls; each tick it pushes 4 into an
  adjacent soul handler that accepts and, where that took nothing, pulls 4 out of one.
- **`soul_surge`** — spends 1 soul for 300 accelerated ticks and random-ticks the block it is
  mounted on four times at a 3% chance a tick, while it is not powered.

Why this is the core family: the soul economy has exactly **one source** (`SLBSoulCap` is
implemented by no class but `SoulLaserBaseBlockEntity`), the pipe only moves souls and the surge only
spends them. No laser base ⇒ no souls anywhere in the mod.

### 1.1 The dependency this port has on the parent

The laser base's gate is an item of **Industrial Foregoing**:
`catalyst.getStackInSlot(0).getItem().equals(ModuleCore.LASER_LENS[11].get())` — the blue lens.
`industrialforegoing` **0.2.4** ships the 16 `<colour>_laser_lens`, the machine frames with
`industrialforegoing:machine_frame/*`, the gears under `c:gears/*` and `plastic` under `c:plastics`,
which is what unblocked this port (the earlier pass stopped at
`blocked(missing-parent-item: industrialforegoing:blue_laser_lens)`,
`ports/_shared/evidence/industrialforegoingsouls/BLOCKED-parent-surface.md`).

`register` resolves that item once, so **a game without the parent refuses the package at load**:

```
[Render thread/ERROR]: Horizon: mod industrialforegoingsouls requested unknown item industrialforegoing:blue_laser_lens
```

That is the mod's own required dependency (upstream declares
`dependencies = [{ modId = "industrialforegoing", type = "required" }]`), and `horizon-mod.toml` has
no dependency field to declare it in, so the refusal is the load. **The in-game leg therefore builds
one game carrying both mods** (`tools/e2e-two-mod.sh`: the launcher's `native-build <folder>` takes a
folder of mod folders); `ports/_shared/evidence/industrialforegoingsouls-e2e/` is its output.

## 2. The family table (from the jar, before any code)

147 entries, 329,233 B: three blocks, no items of its own, one client GUI addon, three recipes, three
loot tables, five tags.

| family | jar evidence | row |
|---|---|---|
| `soul_laser_base` block + BE (bar, warden harvest, 1,350 tank, catalyst slot) | `block/SoulLaserBaseBlock.java`, `block/tile/SoulLaserBaseBlockEntity.java` | **ported and tested** (phases A–D) |
| `soul_network_pipe` block + BE + Titanium `SoulNetwork` | `block/SoulPipeBlock.java`, `block/tile/SoulPipeBlockEntity.java`, `block_network/*` | **ported and tested** (phase E) |
| `soul_surge` block + BE (300-tick soul, random ticks, redstone) | `block/SoulSurgeBlock.java`, `block/tile/SoulSurgeBlockEntity.java` | **ported and tested** (phases F–H), with the two gaps below |
| 3 shaped recipes | `data/*/recipe/{soul_laser_base,soul_network_pipe,soul_surge}.json` | **ported and crafted in game** (phase K) |
| 3 loot tables + `mineable/pickaxe` | `loot_table/blocks/*.json`, `tags/block/mineable/pickaxe.json` | **ported and broken for drops** (phase L) |
| `industrialforegoingsouls:cant_accelerate`, `c:tick_acceleration_disallowed` | `tags/block/*.json` | **ported and read by the tick** (the surge is in both) |
| `laser_drill_ore/echo_shard` + `deep_dark` biome tag | `recipe/laser_drill_ore/echo_shard.json` | **not ported** — the recipe type is the parent's (`industrialforegoing:laser_drill_ore`), and the parent port registers no recipe type; a file naming it is dead data |
| recipe advancements ×3 | `advancement/recipes/misc/*.json` | **ported** (the jar's, `neoforge:conditions` dropped) |
| `SculkSoulTankScreenAddon` (client) | `client/SculkSoulTankScreenAddon.java` | **not ported** — it draws on the parent's Sculk Soul Tank GUI, which the parent port does not ship; there is no screen to add to |
| comparator output (`getAnalogOutputSignal`) | `block/SoulLaserBaseBlock.java:31-39` | **blocked(`analog-output-needs-block-entity-read`)** — the `analog_output` hook is given `&dyn LevelHost`, which has no way to resolve a block entity by position. Formula and test (`LaserBase::analog_output`) are in `runtime.rs`, ready for the hook |
| the surge's block-entity branch (×4 ticks of a machine in front) | `SoulSurgeBlockEntity.java:60-70` | **blocked(`no-tick-another-block-entity`)** — `block_entity_tick` ticks the owning entity only |
| the surge's entity branch (×4 ticks of `LivingEntity`) | `SoulSurgeBlockEntity.java:78-90` | **blocked(`no-entity-tick`)** — no helper ticks an entity |
| config (`SOUL_STORAGE_AMOUNT 1350`, `MAX_PROGRESS 20`, `SOULS_PER_OPERATION 2`, `DAMAGE_PER_OPERATION 4`, `SOUL_TIME 300`, `BLOCK_ACCELERATION_TICK 4`, `RANDOM_TICK_ACCELERATION_CHANCE 0.03`, `SOUL_AMOUNT_PER_PIPE 4`) | `config/*.java` | the values are `runtime.rs` constants; **no config file** (a recorded deviation) |
| waterlogging on the pipe and the surge | `SimpleWaterloggedBlock` | **not ported** — neither the chorus-plant template nor the surge declares `waterlogged`; no fluid state |

## 3. What the port does differently, and why

Each of these is a decision the pin forced; none of them fakes a rule.

1. **The network is the connected pipes, not a Titanium graph.** Horizon has no
   `block_network`/`NetworkManager`, and no capability for a mod's own handler
   (`SoulCapabilities.BLOCK` is NeoForge's). The pipe walks its six neighbours through pipe blocks
   (a `VecDeque` BFS in `lib.rs::walk`), each pipe holds up to 4 souls, and
   `runtime::Network` adds/drains in walk order. Capacity is `4 × pipes`, the same number upstream
   computes (`getMaxSouls = graph.getElements().size() * SOUL_AMOUNT_PER_PIPE`); the difference is
   that upstream keeps the total on the network object and this port keeps it per pipe, which
   survives a save with no network registry. **The surge is not counted as an element** (upstream's
   graph counts it), so a surge-only network has capacity 0 — nothing can fill it either way.
2. **The pipe's handler is found by block entity type, not by capability.** The only handler this
   package ships is the laser base, and its `fill` returns 0 (`SLBSoulCap.fill`), so the pull is the
   whole transfer — the same branch upstream takes against the same handler. A soul handler in
   *another* mod would need a capability API (recorded as an issue-shaped gap, not worked around).
3. **The catalyst slot is a click, not a GUI.** Titanium's screens are the parent's; the slot is
   `BlockPart::use_item_on` (right-click with the lens seats it, taking one out of the hand) and
   `BlockPart::use_without_item` (an empty hand takes it back into the selected slot). Upstream's
   `SidedInventoryComponent` also lets a hopper in; this port has no item capability on the block.
4. **The surge's `enabled` and connection states are not declared.** A block state cannot be
   rewritten from a tick or a `neighbor_changed` (`LevelTicksHost` has no write) without
   `LevelRef::set_block`, which recreates the block entity and would drop the surge's counter. So the
   tick asks `level.has_neighbor_signal` directly — the same rule `neighborChanged` applies one step
   later — and the surge draws as its model on a full-cube shape (no `soul_surge_off`, no connection
   arms).
5. **The pipe's three-value connection enum becomes the engine's pipe template.** 26.3 builds a
   pipe's shape and joins it on placement and every neighbour change for a
   `minecraft:chorus_plant` template with `connects_to`; the jar's `none|block|pipe` collapses to one
   boolean a side, and `soul_network_pipe_side` is drawn for each joined side. The jar's
   `soul_network_pipe_side_to_pipe` (a second arm style) is not carried.
6. **`c:stones` is ours.** 26.3 ships no `c:` item tags, so two of the three recipes would never
   match. The port declares `data/c/tags/item/stones.json` with the 12 vanilla natural stones
   `simplylight` declares, so every port shipping the tag agrees; the e2e crafts with
   `minecraft:stone`, which all four variants in the tree hold.
7. **No config file.** Every number is a `runtime.rs` constant; a server owner cannot retune the
   economy without a new build (`/industrialforegoingsouls` has no settings command).

### Upstream defects fixed

None found. The jar and the source agree on every number, id and tag this slice uses, and the two
shapes that 26.3 spells differently (object-form ingredients, the plural loot `conditions`) are
**dialect**, not defects — `tools/assemble_content.py` re-spells them and `notes` keeps the record.
The one upstream shape that reads like a bug is **not** fixed: a surge facing empty air with no
entity in it does not spend its 300 ticks (`SoulSurgeBlockEntity.java:74-79` decrements only inside
the branches that accelerated something), which is the useful semantics — a soul is not burned on
nothing — and the runtime test `a_surge_facing_nothing_keeps_its_soul` pins it.

## 4. Red → green

The runtime's 17 tests were written first and watched fail:

```
$ cargo test            # before any implementation
test runtime::tests::a_full_bar_needs_twenty_ticks_before_the_first_operation ... FAILED
thread 'runtime::tests::a_full_bar_needs_twenty_ticks_before_the_first_operation' panicked at src/runtime.rs:52:9:
not yet implemented: slice 1
test result: FAILED. 0 passed; 17 failed; 0 ignored; 0 measured; 0 filtered out
```

(`ports/_shared/evidence/industrialforegoingsouls/sdk-test-red.log`, 17 × `not yet implemented`.)
Two of my own expectations were wrong against upstream and were corrected *in the test*, not in the
code: a tank at 1,349 souls still runs an operation (the gate is `soulAmount < 1350` before it) and
clamps to 1,350; and a surge facing nothing keeps its 300 ticks. Then:

```
$ cargo test
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The in-game legs then failed for real reasons and each was traced, not retried: the pipe's network
holds 4 souls with one pipe (so it takes exactly 4 out of the machine, not 4 a tick forever); a
crop's age is in the `state` string, not a `properties` object; a crop two pixels tall cannot be
clicked on its side, so the placement rule is proven on a full block and the acceleration fixture
goes in with its state; and a chorus-plant pipe's six-pixel column cannot be hit by a crosshair
2 blocks out at eye height, so its drop is proven on a pedestal at eye level.

## 5. In-game evidence (fail-capable)

`ports/_shared/evidence/industrialforegoingsouls-e2e/`, all four through
`tools/e2e-two-mod.sh` (one game carrying this port **and** its parent):

| leg | verdict | what it proves |
|---|---|---|
| `e2e-launcher.log` | **`PASS` + `EXIT:0`** | the whole scenario below |
| `e2e-launcher-control.log` | `FAIL: the game refused industrialforegoingsouls:soul_laser_base` + `EXIT:1` | the same script on the same Nightly **without** the package |
| `e2e-launcher-selftest.log` | `FAIL: selftest: the soul tank is 0, expected anything but 0` + `EXIT:1` | the lens differential can fail |
| `e2e-launcher-selftest-grow.log` | `FAIL: selftest: the control wheat's age is 0, expected it above 0` + `EXIT:1` | the acceleration differential can fail |

`python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` →
`logs=4 ok=4 missing-exit=0 bad-exit=0`.

**The lens differential**, the assertion the whole mod hangs on, from the transcript:

```
Soul Laser Base at [194, 101, 0]: souls 0/1350,  progress 11/20, lens none, warden 500.0, t=209
  60 ticks, no lens: souls 0, the warden at 500.0 health
Soul Laser Base at [194, 101, 0]: souls 8/1350,  progress 17/20, lens industrialforegoing:blue_laser_lens, warden 484.0, t=353
  the lens: 4 operations, 8 souls, warden 484.0 health
Soul Laser Base at [194, 101, 0]: souls 10/1350, progress 2/20,  lens none, warden 480.0, t=468
  the lens taken back: the tank stays at 10 souls for 60 ticks
```

and the surge's, with `random_tick_speed 0` so nothing grows on its own:

```
Soul Surge at [202, 102, -1]: network 3/4, wait 293/300, facing north, powered false, t=674
Soul Surge at [202, 102, -1]: network 2/4, wait 200/300, facing north, powered false, t=1087
  after 400 ticks: accelerated wheat age 4, control age 0, surge wait 200/300
Soul Surge at [202, 102, -1]: network 2/4, wait 185/300, facing north, powered true,  t=1114
  powered: wait frozen at 172; unpowered again
```

## 6. Coverage table (family → how it is proven)

| family | row |
|---|---|
| laser base places, reads back, refuses an unknown id | **tested** — phase A (`ctl give`/`setblock` refusals, the parsed readout) |
| 20-tick bar, warden harvest, 2 souls an operation, 1,350 tank | **tested** — phase C: 4 operations = 16 damage = 8 souls, `souls == 2 × operations` |
| the lens gate (blue only, none without) | **tested** — phases B and D, the differential above |
| the 1,350 cap | **tested** — `the_tank_stops_at_1350` (unit) |
| the comparator output | **blocked(`analog-output-needs-block-entity-read`)** — formula unit-tested |
| pipe network 4 a pipe, 4-pull from the machine | **tested** — phase E: 50 → 46 with one pipe (4/4), 46 → 42 with two (8/8) |
| the pipe's shape and joins | **carried** — `chorus_plant` template + `connects_to` (not asserted in game; the state string shows `east=true` next to its neighbour) |
| surge placement (`state_for_placement` → `facing` from the clicked face) | **tested** — phase F: a top-face placement reads `facing=up` |
| surge spends one soul for 300 ticks | **tested** — phase G: network 3 → 2, `wait` 293 → 200 over 400 ticks |
| surge random-ticks the block it is mounted on | **tested** — phase G: wheat age 4 with `random_tick_speed 0`, control 0 |
| surge disabled while powered | **tested** — phase H: `powered true`, `wait` frozen, then false again |
| the three recipes | **tested** — phase K: ids and counts (8 pipes), ingredients consumed, grid empty; a vanilla stand-in grid crafts nothing |
| loot tables | **tested** — phase L: each block breaks in survival and drops itself |
| tags (`cant_accelerate`, `c:tick_acceleration_disallowed`, `mineable/pickaxe`, `soul_connects`, `c:stones`) | **tested** as they are consumed (the surge's own block is in both cant-accelerate tags, the recipes use `c:stones`, the pickaxe tag is what the survival break needs) |
| `laser_drill_ore/echo_shard` | **not ported** — parent recipe type |
| Sculk Soul Tank screen addon | **not ported** — parent GUI |
| surge BE / entity branches | **blocked** — `no-tick-another-block-entity`, `no-entity-tick` |
| config file | **not ported** — constants |

## 7. Gaps (issue-shaped, for the ledger)

- `blocked(analog-output-needs-block-entity-read)` — `BlockPart::analog_output` gets
  `&dyn LevelHost`; a comparator read of a block entity's own compound needs a host that resolves
  one by position.
- `blocked(no-tick-another-block-entity)` — the surge's headline use (accelerating a machine) has no
  helper; `block_entity_tick` ticks only its own entity.
- `blocked(no-entity-tick)` — `LivingEntity.tick()` ×4 has no route.
- `blocked(no-mod-capability)` — the pipe cannot see a soul handler in another mod; the four
  `block_entity` read APIs are the whole surface.
- `blocked(no-block-state-write-outside-set-block)` — for a machine that keeps state in its block
  entity, rewriting its own state recreates it.

## 8. The ladder (evidence index)

`ports/_shared/evidence/industrialforegoingsouls/`:

| step | command | result | log |
|---|---|---|---|
| red | `cargo test` | 0 passed, **17 failed** (`not yet implemented: slice 1`) | `sdk-test-red.log` |
| tests | `cargo test` | **17 passed, 0 failed** | `sdk-test.log` |
| fmt + lints | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` | `FMT-CLEAN`, `CLIPPY-CLEAN` | `sdk-clippy.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `CMP-IDENTICAL`, sha256 `9169cbc4…533`, 59 entries, no `Cargo.lock` | `sdk-build.log` |
| check | `cargo horizon-mod check industrialforegoingsouls.hmod` | `industrialforegoingsouls 0.2.0 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, 77,206 B, exit 0 | `sdk-check.log` (prints the sha) |
| fits (pin) | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, exit 0 | `sdk-fits.log` (prints the sha) |
| fits (other) | `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, exit 0 | `sdk-fits.log` |
| content | `python3 tools/assemble_content.py --check` | `CONTENT OK: 53 files` | — |
| package gate | `python3 ports/tools/verify-sdk-package.py industrialforegoingsouls` | **1/1** | — |
| in-game | `tools/e2e-two-mod.sh run` | `PASS` + `EXIT:0` | `industrialforegoingsouls-e2e/e2e-launcher.log` |
| control | the same script on `GAME=$CONTROL_GAME` | `FAIL` + `EXIT:1` | `…/e2e-launcher-control.log` |
| selftest ×2 | `SELFTEST=1`, `SELFTEST=grow` | `FAIL` + `EXIT:1` each | `…/e2e-launcher-selftest*.log` |
| log gate | `python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` | `logs=4 ok=4 missing-exit=0 bad-exit=0` | — |

## 9. Files

```
Cargo.toml, horizon-mod.toml, LICENSE, README.md, .gitignore
industrialforegoingsouls.hmod     delivered package (77,206 B)
src/lib.rs                        the one impl Hooks + the three Parts
src/runtime.rs                    the pure economy, 17 tests
assets/industrialforegoingsouls/  blockstates, models, item definitions, block textures, lang
data/industrialforegoingsouls/    block declarations, creative tab, loot, recipes, tags, advancements
data/c/…, data/minecraft/tags/…   this port's declarations of the convention tags it needs
tools/assemble_content.py         jar -> assets/data, re-spelled for 26.3 (`--check` proves it)
tools/craft_recipe.py             the three recipes + the stand-in control, in a real table
tools/parse_info.py               `/industrialforegoingsouls info` -> shell fields
tools/e2e.sh                      the in-game scenario
tools/e2e-two-mod.sh              builds the two-mod game (this port + its parent) and runs it
notes.md, FINDINGS.md, VALIDATION.md
```
