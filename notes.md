# `industrialforegoingsouls` — Industrial Foregoing: Souls 0.2.3

The three blocks that are one soul economy, ported from **Industrial Foregoing: Souls 1.21.1-1.10.7**
into Rust for Horizon 26.3 at the campaign pin.

| | |
|---|---|
| class · tier | **content · machines** (`ports/ledger.csv`) |
| licence | **MIT**, read from the jar's `META-INF/neoforge.mods.toml` (`audit_licences.py --mod industrialforegoingsouls --fetch` → `declared MIT / effective MIT`), rank **0** |
| approach (`horizon/docs/modpacks.md`) | **agent port** of the behaviour, with the jar's content carried (models, blockstates, textures, recipes, loot, tags) |
| pin | Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`, horizon-mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| upstream | `InnovativeOnlineIndustries/Industrial-Foregoing-Souls`, commit `bf736d2f1140da25747ab97d8c160b44d8a0bb7f` (2026-01-10, the jar's `1.10.7`); the jar is `ports/_shared/licence-cache/industrial-foregoing-souls-1.21.1-1.10.7.jar`, sha256 `f4fb3640…67ec` |
| package | `industrialforegoingsouls.hmod`, **101,380 B**, sha256 `68b1485efc1539b665aeb623e986d042bb73626723bc3d9cd4c6b1d3124c88b8`, 63 entries, no packed `Cargo.lock` |
| registry | **first publication**, open on PR #1605; that PR publishes **`0.2.2`** (0.2.0/0.2.1 were dropped at the reviewer's rule — neither carried the declared dependency or the MIT notice). This is `0.2.3` appended to #1605, title `Add industrialforegoingsouls 0.2.3` (the id is not on `main`) |

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-review

## 1. The slice

The mod is **one mechanic in three blocks**, and a package with only one of them is inert, so the
first slice is all three:

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
`dependencies = [{ modId = "industrialforegoing", type = "required" }]`). **0.2.2 declares it**, with
the pin's schema (`crates/horizon-mod-package` at the pin: `version`, `optional`, `side`, `reason`,
`url`; reason ≤ 512 bytes, url `https://`):

```toml
[dependencies]
industrialforegoing = { version = ">=0.2.7", reason = "…", url = "https://github.com/AdLf0208/industrialforegoing" }
```

A bundle without the parent is now refused **by name** before anything starts, rather than failing
later at registration:

```
error: industrialforegoingsouls requires missing mod industrialforegoing (The Soul Laser Base seats
industrialforegoing:blue_laser_lens and the carried laser_drill_ore/echo_shard recipe is the
parent's type, registered from 0.2.7. https://github.com/AdLf0208/industrialforegoing)
```

(`ports/_shared/evidence/industrialforegoingsouls/sdk-dependency-refusal.log`, one-module
`native-build`, exit 1.) **The in-game leg therefore builds one game carrying both mods**
(`tools/e2e-two-mod.sh`: the launcher's `native-build <folder>` takes a folder of mod folders) and
the build compiles `industrialforegoing (1 of 2)` then `industrialforegoingsouls (2 of 2)`;
`ports/_shared/evidence/industrialforegoingsouls-e2e/` is its output.

### 1.2 The 0.2.1 slice: the rest of the jar's own surface

Two families the 0.2.0 package left out, both in the jar and both `machines` shaped:

1. **The efficiency augment on the Soul Laser Base.** The 0.2.0 coverage table did not list it at
   all — it was a family neither tested, not ported nor blocked — while `FINDINGS.md` recorded it
   (§`SoulLaserBaseBlockEntity`: "`canAcceptAugment` refuses speed; efficiency is accepted (the bar
   shortens)"). The rule, re-read from the upstream source at `bf736d2f` for this slice: `onWork`
   ends with
   `int maxProgress = (int) Math.floor(MAX_PROGRESS * (hasAugmentInstalled(EFFICIENCY) ? AugmentWrapper.getType(…, EFFICIENCY) : 1)); work.setMaxProgress(maxProgress);`
   and `canAcceptAugment` returns false for a stack carrying `Speed`. So the **first** bar is always
   the plain twenty and every bar after it is `floor(20 × efficiency)` — 16 for the tier 2 addon
   (0.8), 18 for tier 1 (0.9) — and speed never seats.
2. **`industrialforegoing:laser_drill_ore/echo_shard` and its `deep_dark` biome tag.** The 0.2.0 row
   read "**not ported** — the recipe type is the parent's (`industrialforegoing:laser_drill_ore`),
   and the parent port registers no recipe type; a file naming it is dead data". The parent's 0.2.7
   (published as #1665; 0.2.8 by the 0.2.1 legs, same type) registers it, and the parent's own e2e
   then loaded **this package's** file and tag from a datapack. The file is now carried by this package and the parent's ore laser base runs
   it: the difference a user sees is that an echo shard is obtainable from the parent's laser in the
   deep dark.

The Sculk Soul Tank screen addon stays **not ported** (the parent ships no Sculk Soul Tank GUI to
add to), and the two acceleration branches and the comparator stay `blocked` for the same hook gaps
as 0.2.0 — nothing in this pin changed for them.

### 1.3 The 0.2.2 review fixes (reviewer `levzzz5154`, static review of `e2062f35`)

Four findings, fixed here without a new family:

1. **The parent is declared** (above).
2. **The catalyst lens is a swap.** A second blue lens used to overwrite the saved one and eat the
   new item, voiding the old one. `use_item_on` now reserves a destination first
   (`runtime::replacement_slot`: the hand's slot when the held stack empties, else the first free
   hotbar/main/off-hand slot, `None` when the inventory is full so the click is refused), validates
   the old item id, then merges, shrinks the hand and hands the old lens back. The broken-machine
   carry (`block_drops` → fresh `minecraft:block_entity_data` → `on_block_placed_by`) was already in
   the 0.2.2 tree; this run proves it in game for the first time (phase P).
3. **The pipe walk is bounded and once a tick.** `runtime::grow_component` visits each cell through a
   `HashSet` (no `Vec::contains`) and stops at `MAX_NETWORK_PIPES = 4096`;
   `runtime::NetworkCache` keys a walked component on the level and its game time, so the second
   pipe of a component in one tick reuses it (`with_pipe_network`). Only pipes whose souls moved are
   written back. A placement or a break drops the cache. Tests: a 200-pipe line is walked once with
   ≤ 6 probes a pipe and the other 199 pipes ask the level nothing; a 5,000-pipe line stops at 100
   with a 100 budget.
4. **The MIT notice is upstream's verbatim**, `Copyright (c) 2021 Buuz135, Rid`.

### 1.4 The 0.2.3 slice: waterlogging on the pipe and the surge

The gap row this closes, verbatim from §2 as 0.2.2 left it:

> | waterlogging on the pipe and the surge | `SimpleWaterloggedBlock` | **not ported** — neither the
> chorus-plant template nor the surge declares `waterlogged`; no fluid state |

Upstream's `SoulPipeBlock` **is** a `SimpleWaterloggedBlock` and `SoulSurgeBlock` declares the
`WATERLOGGED` property (`FINDINGS.md`). What the pin can express, measured before writing code:

| upstream | this pin | evidence |
|---|---|---|
| the `WATERLOGGED` state property | **yes** — a declared `state_properties: { "waterlogged": {"type":"boolean"} }` on a template that lacks it | `horizon/docs/mods.md` (state properties); `data/…/horizon/block/*.json` after this slice |
| a fluid state for the water it holds | **yes, and generic**: the engine derives a state's fluid from the **property name** (`prop("waterlogged") == Some("true") => (Fluid::Water, 8, false)`) and `is_waterlogged` is `table.bool(state, props.waterlogged)` — so a mob, fire, the minimap and the LOD all see the water | `crates/horizon-items/src/blocks.rs:431`, `crates/horizon-world/src/level/query.rs:117` |
| the block's interface (`instanceof SimpleWaterloggedBlock`) | **no** — a content block's interface set is *copied from its template* (`chorus_plant`, `iron_block`: neither has it), `connects_to` rewrites the class only, and a template with both six-sided pipe joining and the interface does not exist | `crates/horizon-world/src/level/buckets.rs:50`, `crates/horizon-world/src/level/fluids.rs:76`, `crates/horizon-client/src/mod_content.rs` (`renames`/`states` clone the template's row) |
| `BucketItem.emptyContents`/`BucketPickup` reaching the block | **through the block's own click** — exactly how this port already seats the lens and the augment (it ships no GUI): `BlockPart::use_item_on` writes the state and trades the bucket, with the vanilla bucket sound and `minecraft:fluid_place`/`minecraft:fluid_pickup` | `src/lib.rs` `waterlog_click`, `src/runtime.rs` `water_click`/`Waterlogged`/`bucket_destination` |

So the slice ships the **state**, the engine's **fluid facts** for it and the **two bucket
interactions**. The interface-gated half is recorded as a gap (§7):
`blocked(no-waterlogged-interface)` — a **flowing fluid** does not fill the block, a **dispenser**
does not, and the jar's `getStateForPlacement` override (placing the block *into* water) could not
be verified from the findings, so the port does not invent it.

The drain hands its water back to the **inventory**, not the hand: this pin reverts a water bucket
written into the *held* slot from a click (measured 2026-11-23 with `set_inventory_slot` **and** with
`give_stack`; the same write to a main-inventory slot persists, and a plain item in the held slot
persists too). So `runtime::bucket_destination` picks the first free slot that is not the clicking
hand, the off hand after it, and a full inventory refuses the drain rather than losing the water.
Vanilla's `ItemUtils.createFilledResult` puts it in the hand; a player sees the water appear in the
inventory instead. That is the one user-visible difference of this slice.

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
| `laser_drill_ore/echo_shard` + `deep_dark` biome tag | `recipe/laser_drill_ore/echo_shard.json`, `tags/worldgen/biome/deep_dark.json` | **ported and run in game** (phase M2): the parent's ore laser base resolves it at depth 0 in `deep_dark` and yields one echo shard |
| the efficiency augment (`AugmentTypes.EFFICIENCY`, two `titanium:augments` slots) | `SoulLaserBaseBlockEntity.canAcceptAugment`, `onWork` | **ported and tested** (phases D2–D3) |
| recipe advancements ×3 | `advancement/recipes/misc/*.json` | **ported** (the jar's, `neoforge:conditions` dropped) |
| `SculkSoulTankScreenAddon` (client) | `client/SculkSoulTankScreenAddon.java` | **not ported** — it draws on the parent's Sculk Soul Tank GUI, which the parent port does not ship; there is no screen to add to |
| comparator output (`getAnalogOutputSignal`) | `block/SoulLaserBaseBlock.java:31-39` | **blocked(`analog-output-needs-block-entity-read`)** — the `analog_output` hook is given `&dyn LevelHost`, which has no way to resolve a block entity by position. Formula and test (`LaserBase::analog_output`) are in `runtime.rs`, ready for the hook |
| the surge's block-entity branch (×4 ticks of a machine in front) | `SoulSurgeBlockEntity.java:60-70` | **blocked(`no-tick-another-block-entity`)** — `block_entity_tick` ticks the owning entity only |
| the surge's entity branch (×4 ticks of `LivingEntity`) | `SoulSurgeBlockEntity.java:78-90` | **blocked(`no-entity-tick`)** — no helper ticks an entity |
| config (`SOUL_STORAGE_AMOUNT 1350`, `MAX_PROGRESS 20`, `SOULS_PER_OPERATION 2`, `DAMAGE_PER_OPERATION 4`, `SOUL_TIME 300`, `BLOCK_ACCELERATION_TICK 4`, `RANDOM_TICK_ACCELERATION_CHANCE 0.03`, `SOUL_AMOUNT_PER_PIPE 4`) | `config/*.java` | the values are `runtime.rs` constants; **no config file** (a recorded deviation) |
| waterlogging on the pipe and the surge | `SimpleWaterloggedBlock` | **ported and tested** (phase R): the `waterlogged` state on both, the engine's water fluid state, a water bucket fills it and an empty bucket drains it; the interface itself (a flowing fluid, a dispenser, placement into water) stays `blocked(no-waterlogged-interface)` |

## 3. What the port does differently, and why

Each of these is a decision the pin forced; none of them fakes a rule.

1. **The network is the connected pipes, not a Titanium graph.** Horizon has no
   `block_network`/`NetworkManager`, and no capability for a mod's own handler
   (`SoulCapabilities.BLOCK` is NeoForge's). The pipe walks its six neighbours through pipe blocks
   (`runtime::grow_component`: set-based visitation, one probe a cell, cut at
   `MAX_NETWORK_PIPES = 4096`), and `runtime::NetworkCache` walks each connected component **once a
   tick** so every other pipe of it reuses the positions and souls; only pipes whose souls moved
   are written back. Each pipe holds up to 4 souls and `runtime::Network` adds/drains in walk order.
   Capacity is `4 × pipes`, the same number upstream computes (`getMaxSouls =
   graph.getElements().size() * SOUL_AMOUNT_PER_PIPE`); the difference is that upstream keeps the
   total on the network object and this port keeps it per pipe, which survives a save with no
   network registry. **The surge is not counted as an element** (upstream's graph counts it), so a
   surge-only network has capacity 0 — nothing can fill it either way.
2. **The pipe's handler is found by block entity type, not by capability.** The only handler this
   package ships is the laser base, and its `fill` returns 0 (`SLBSoulCap.fill`), so the pull is the
   whole transfer — the same branch upstream takes against the same handler. A soul handler in
   *another* mod would need a capability API (recorded as an issue-shaped gap, not worked around).
3. **The catalyst slot is a click, not a GUI.** Titanium's screens are the parent's; the slot is
   `BlockPart::use_item_on` (right-click with the lens seats it, taking one out of the hand) and
   `BlockPart::use_without_item` (an empty hand takes it back into the selected slot). A second lens
   is a swap, as the GUI's cursor would be: the old lens goes back into the hand that empties (or
   the first free slot), and a full inventory refuses the click rather than voiding it
   (`runtime::replacement_slot`). Upstream's `SidedInventoryComponent` also lets a hopper in; this
   port has no item capability on the block.
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
8. **The augment slots are a click, not a GUI.** Titanium's two-slot augment inventory is the block
   entity's saved `augments` list (two compounds, each the item's `titanium:augments` value, by
   position); the machine's `use_item_on` seats one and takes one out of the hand, an empty hand
   gives it back, and a speed addon is refused and stays in the hand (`canAcceptAugment`). Because
   the item id is not in the component, the seat also saves `augment_item`, which is what lets a
   click hand the addon back — with a GUI the slot itself would carry it.
   **Measured platform behaviour, not assumed:** the engine also calls `use_without_item` after an
   item's `use_item_on` returns pass, with the stack still in the hand, so the empty-hand path reads
   the hand first (`inventory_slots`); without that guard a refused speed addon's click took the
   seated efficiency addon back out (first run of phase D2, 2026-11-22).
9. **The ore recipe is the parent's type, carried in this package's namespace.** The file is the
   jar's own `laser_drill_ore/echo_shard.json`; its object-form `catalyst`/`output` are that recipe
   type's serializer's dialect (not a shaped recipe's ingredient shape), and 26.3 drops a recipe
   whose *type* no package registers. The parent package registers it, and a game without the parent
   refuses this package at load anyway (the lens), so the pair is always together. The jar's other 41
   `laser_drill_ore` files name ores no package here ships and are not carried.
10. **Waterlogging is the state plus a click, and the drained bucket goes to the inventory.** A
    content block cannot attach `SimpleWaterloggedBlock` (its interfaces are its template's), so the
    block's own `use_item_on` is where the water goes in and out; the engine still reads the water
    from the state. The drain cannot put the full bucket **in the hand** — the pin reverts a water
    bucket written into the held slot from a click (measured, §1.4) — so it takes the first free
    slot, and a full inventory refuses the drain. A flowing fluid and a dispenser do not waterlog
    the block (§7).

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

### 4.1 Slice 2's red → green (0.2.1)

The five new rules were written first, with `bar_max` at `todo!()` and the tick taking the augment
as an `Option`, so the 17 carried tests still ran unchanged and only the new family failed:

```
$ cargo test
test runtime::tests::an_efficiency_augment_shortens_the_bar_after_the_first_operation ... FAILED
test runtime::tests::a_tier_one_efficiency_augment_gives_an_eighteen_tick_bar ... FAILED
test runtime::tests::ten_operations_of_a_sixteen_tick_bar_store_twenty_souls ... FAILED
test runtime::tests::a_saved_bar_is_kept_and_a_zero_one_is_not ... FAILED
test runtime::tests::the_bar_maximum_is_the_floor_of_twenty_times_efficiency ... FAILED
thread 'runtime::tests::the_bar_maximum_is_the_floor_of_twenty_times_efficiency' panicked at src/runtime.rs:44:5:
not yet implemented: slice 2: (int) Math.floor(MAX_PROGRESS * efficiency)
test result: FAILED. 17 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out
```

(`ports/_shared/evidence/industrialforegoingsouls/sdk-test-red-slice2.log`.) One of my own
expectations was wrong again and was corrected in the test: a 160-tick window that *contains* the
plain first bar stores 22 souls (1 + 10), not 20, so the test starts from a machine already at bar
16 and the `160 ticks → 10 operations` claim is the one pinned. Green:

```
$ cargo test
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The content red was the carry tool's own: `python3 tools/assemble_content.py --check` answered
`differs or missing:` for the three files before they existed, and `CONTENT OK: 56 files` after.
The in-game legs found two real defects that the unit tests could not: the `use_without_item`
fall-through (§3 item 8), and phase E measuring the pipe against a machine that was still harvesting
because D3 left the lens seated (fixed by taking the lens out first — the tank must be still for a
transfer to be readable).

### 4.2 The 0.2.2 review fixes' red → green

The two new pure rules were written first, against `todo!()` stubs, with the 29 carried tests
running unchanged; only the review's rules failed:

```
$ cargo test
test runtime::tests::a_cached_component_keeps_the_ticks_transfer ... FAILED
test runtime::tests::a_network_walk_stops_at_the_budget ... FAILED
test runtime::tests::a_replaced_lens_goes_back_into_the_hand_or_the_first_free_slot ... FAILED
test runtime::tests::a_two_hundred_pipe_line_is_walked_once_a_tick ... FAILED
thread 'runtime::tests::a_replaced_lens_goes_back_into_the_hand_or_the_first_free_slot' panicked at src/runtime.rs:138:5:
not yet implemented: slice 0.2.2 review: the replaced lens needs a slot before the new one is consumed
thread 'runtime::tests::a_two_hundred_pipe_line_is_walked_once_a_tick' panicked at src/runtime.rs:149:5:
not yet implemented: slice 0.2.2 review: the pipe walk visits each cell once and stops at the budget
test result: FAILED. 29 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
```

(`ports/_shared/evidence/industrialforegoingsouls/sdk-test-red-review.log`.) Green:

```
$ cargo test
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

(`sdk-test.log`.) The dependency red is the launcher's refusal
(`sdk-dependency-refusal.log`), and the in-game reds are the two new selftest legs below.

### 4.3 The 0.2.3 slice's red → green (waterlogging)

The four waterlog rules were written first, against `todo!()` stubs, with the 33 carried tests
running unchanged; only the new family failed:

```
$ cargo test
test runtime::tests::a_water_bucket_fills_a_dry_block_and_refuses_a_full_one ... FAILED
test runtime::tests::an_empty_bucket_drains_a_waterlogged_block_and_refuses_a_dry_one ... FAILED
test runtime::tests::anything_but_a_bucket_is_not_a_water_click ... FAILED
test runtime::tests::only_the_property_true_is_waterlogged ... FAILED
thread 'runtime::tests::a_water_bucket_fills_a_dry_block_and_refuses_a_full_one' panicked at src/runtime.rs:195:5:
not yet implemented: slice 0.2.3: a full bucket fills a dry machine, an empty one drains a waterlogged one
test result: FAILED. 33 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
```

(`ports/_shared/evidence/industrialforegoingsouls/sdk-test-red-waterlog.log`, four `not yet
implemented: slice 0.2.3`.) The drain's destination rule was written second, when the in-game leg
showed the pin reverting a water bucket in the held slot, and its own red came first too:

```
$ cargo test
test runtime::tests::a_drained_bucket_lands_in_the_first_free_slot_but_not_the_hand ... FAILED
thread 'runtime::tests::a_drained_bucket_lands_in_the_first_free_slot_but_not_the_hand' panicked at src/runtime.rs:218:5:
not yet implemented: slice 0.2.3: the drain's full bucket needs a free slot that is not the held one
test result: FAILED. 37 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

(`sdk-test-red-waterlog-drain.log`.) Green:

```
$ cargo test
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

(`sdk-test.log`.) The content red was the carry tool's own (`assemble_content.py --check` answered
`writes 56 files`/`CONTENT OK: 56 files` after the two declarations and the surge's 12-variant
blockstate), and the in-game reds were the leg's: phase G's 400-tick window and the `block_prop`
helper (below), then the drain's bucket.

**Two harness defects this slice found and fixed** (both are the scenario's, not the port's):

- **`block_prop` only ever answered for the first property of a state.** `sed "s/.*\[$4=..."` on
  `soul_network_pipe[down=false,…,waterlogged=false,west=false]` printed nothing, and phase G had
  only ever asked for `facing`/`age`, which happen to be first. It now splits the bracket body and
  matches the name with `awk`.
- **Phase G's 400-tick window was a 6.4 % coin flip.** The surge's tick lands about one game tick in
  four (measured: 95 decrements per 414 game ticks), so 400 ticks is ~95 surge ticks and a 3 % roll
  has a 6.4 % chance of no hit at all — the first 0.2.3 run hit exactly that (`age 0`). The window
  is now 1,200 ticks (~275 surge ticks, ~0.02 %), and the same leg then grew the wheat to age 7.

## 5. In-game evidence (fail-capable)

`ports/_shared/evidence/industrialforegoingsouls-e2e/`, all through `tools/e2e-two-mod.sh` (one game
carrying this port **and** its parent, `industrialforegoing` 0.2.10 when these legs ran). The 0.2.0
legs are kept with a `-0.2.0` suffix and the 0.2.1 ones with `-0.2.1`; the unsuffixed logs are this
0.2.3 run:

| leg | verdict | what it proves |
|---|---|---|
| `e2e-launcher.log` (0.2.3) | **`PASS` + `EXIT:0`** | the whole 0.2.3 scenario, phases A–N, D2–D3, R, K–L, P–Q and M2 |
| `e2e-launcher-control.log` | `FAIL: the game refused industrialforegoingsouls:soul_laser_base` + `EXIT:1` | the same script on the same Nightly **without** the package |
| `e2e-launcher-selftest.log` | `FAIL: selftest: the soul tank is 0, expected anything but 0` + `EXIT:1` | the lens differential can fail |
| `e2e-launcher-selftest-augment.log` | `FAIL: selftest: the shortened bar gained 22 souls, expected no more than the plain bar's 16` + `EXIT:1` | the augment's rate differential can fail |
| `e2e-launcher-selftest-laser.log` | `FAIL: selftest: the base yielded minecraft:echo_shard x1, expected an empty output slot` + `EXIT:1` | the ore differential can fail |
| `e2e-launcher-selftest-grow.log` | `FAIL: selftest: the control wheat's age is 0, expected it above 0` + `EXIT:1` | the acceleration differential can fail |
| `e2e-launcher-selftest-carry.log` | `FAIL: selftest: the restored tank is 30, expected 0` + `EXIT:1` | the carried contents are real, not a stale machine |
| `e2e-launcher-selftest-replace.log` | `FAIL: selftest: the lens the swap returned is 1, expected 0` + `EXIT:1` | the replaced lens is returned, not voided |
| `e2e-launcher-selftest-waterlog.log` | `FAIL: selftest: the pipe's waterlogged state is true, expected false` + `EXIT:1` | the water the bucket puts in is real, not a readout |

`python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` →
`logs=16 ok=16 missing-exit=0 bad-exit=0` (four 0.2.0, three 0.2.1, and the nine unsuffixed legs of
this run: the green one, the control and the seven selftests).

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
Soul Surge at [202, 102, -1]: network 3/4, wait 293/300, facing north, waterlogged false, powered false, t=1068
Soul Surge at [202, 102, -1]: network 0/4, wait 0/300,   facing north, waterlogged false, powered false, t=2336
  after 1200 ticks: accelerated wheat age 7, control age 0, surge wait 0/300
```

**The waterlogging transcript**, phase R of the 0.2.3 leg — the state, the bucket, the network and
the surge, with the dry read as the counterfactual:

```
  Soul Pipe at [208, 101, 3]: network 0/4, waterlogged false, t=2537
  a fresh pipe is dry (waterlogged false), network 0/4
  Soul Pipe at [208, 101, 3]: network 0/4, waterlogged true, t=2581
  hand after the pour: {"minecraft:bucket":1}
  a water bucket: waterlogged true, the hand now holds the empty bucket
  Soul Laser Base at [207, 101, 3]: souls 46/1350, progress 10/20, lens none, augments efficiency 1.00, warden none, t=2614
  Soul Pipe at [208, 101, 3]: network 4/4, waterlogged true, t=2629
  the waterlogged pipe pulled the seeded 50 down to 46 into a 4/4 network
  Soul Pipe at [208, 101, 3]: network 4/4, waterlogged false, t=2692
  hand after the drain: {"minecraft:water_bucket":1}
  an empty bucket: waterlogged false, the water bucket back in the inventory
  Soul Surge at [204, 101, -6]: network 3/4, wait 292/300, facing north, waterlogged true, powered false, t=2730
  Soul Surge at [204, 101, -6]: network 3/4, wait 256/300, facing north, waterlogged true, powered false, t=2766
  a waterlogged surge reads its state back, spends a soul and runs (292 -> 256/300)
  Soul Surge at [202, 101, -6]: network 0/0, wait 0/300, facing north, waterlogged false, powered false, t=2788
  the laser base refuses a waterlogged state by the game's own parser
```

The seed's own reply (`Soul Laser Base at [207, 101, 3]: souls 50/1350.`) is in the log above the
readback, so `50 → 46` is the pipe's four, and no warden is anywhere near that machine (it has no
lens either): the only writer of that tank is the pipe.

**The augment differential**, from the 0.2.1 transcript — the bar's own denominator moves, and the
rate with it:

```
Soul Laser Base at [194, 101, 0]: souls 12/1350, progress 5/20, lens none, augments efficiency 0.80, warden 476.0, t=518
Soul Laser Base at [194, 101, 0]: souls 12/1350, progress 5/16, lens none, augments efficiency 0.80, warden 476.0, t=550
  efficiency 0.80 seated; the bar is 16 after one operation
Soul Laser Base at [194, 101, 0]: souls 12/1350, progress 6/16, lens none, augments efficiency 0.80, warden 476.0, t=597
  a speed addon is refused and stays in the hand
Soul Laser Base at [194, 101, 0]: souls 18/1350, progress 4/16, lens industrialforegoing:blue_laser_lens, augments efficiency 0.80, warden 464.0, t=679
Soul Laser Base at [194, 101, 0]: souls 40/1350, progress 8/16, lens industrialforegoing:blue_laser_lens, augments efficiency 0.80, warden 420.0, t=851
  efficiency 0.80: bar 16, 22 souls in the window (the plain bar fits 8 operations)
```

**The carried contents**, phase P of the 0.2.2 transcript — the item's own `minecraft:block_entity_data`
and the machine it places back as:

```
  before the break: souls 30, progress 10/16, lens industrialforegoing:blue_laser_lens, efficiency 0.80
  Player07876 has the following entity data: [… {components: {"minecraft:block_entity_data": {bar: 16,
    augment_item: "industrialforegoing:efficiency_addon_tier_2", souls: 30, progress: 12,
    id: "industrialforegoingsouls:soul_laser_base", lens: "industrialforegoing:blue_laser_lens",
    augments: [{Efficiency: 0.8d}, {}]}}, count: 1, Slot: 1b, id: "…soul_laser_base"}]
  the drop carries souls 30, progress 12/16, lens industrialforegoing:blue_laser_lens, efficiency 0.8
  placed back: souls 30, progress 4/16, lens industrialforegoing:blue_laser_lens, efficiency 0.80
  … the empty machine's item: no components block, and it places as a fresh 12/20 machine
```

**The lens swap**, phase Q — the counterfactual is `SELFTEST=replace` above:

```
  Soul Laser Base at [190, 101, 13]: … lens industrialforegoing:blue_laser_lens …
  a second lens: the machine keeps one, the first is back in the inventory (held 1)
```

**The ore differential**, phase M2 — the parent's machines, this package's data:

```
Laser Drill at [200, 102, 6]: 0/10000 FE, progress 43/50, target [200, 101, 6], augments speed 1 efficiency 1.00, facing up, t=2266
Ore Laser Base at [200, 101, 6]: progress 0/100, depth 101, biome minecraft:the_end, dimension minecraft:the_end, recipe none, weight 0, recipes 11/11, lens empty, output empty, augments speed 1 efficiency 1.00, t=2271
  11/11 recipes; no match in minecraft:the_end
Ore Laser Base at [200, 101, 6]: progress 0/100, depth 0, biome minecraft:deep_dark, dimension minecraft:the_end, recipe industrialforegoingsouls:laser_drill_ore/echo_shard, weight 1, recipes 11/11, lens empty, output empty, augments speed 1 efficiency 1.00, t=2287
  minecraft:deep_dark at depth 0: industrialforegoingsouls:laser_drill_ore/echo_shard, weight 1
Laser Drill at [200, 102, 6]: 9000/10000 FE, progress 39/50, target [200, 101, 6], augments speed 1 efficiency 1.00, facing up, t=2304
Ore Laser Base at [200, 101, 6]: progress 0/100, depth 0, biome minecraft:deep_dark, dimension minecraft:the_end, recipe industrialforegoingsouls:laser_drill_ore/echo_shard, weight 1, recipes 11/11, lens empty, output minecraft:echo_shard x1, augments speed 1 efficiency 1.00, t=2311
  one base bar for the drill's 1,000 FE yielded minecraft:echo_shard x1
```

## 6. Coverage table (family → how it is proven)

| family | row |
|---|---|
| laser base places, reads back, refuses an unknown id | **tested** — phase A (`ctl give`/`setblock` refusals, the parsed readout) |
| 20-tick bar, warden harvest, 2 souls an operation, 1,350 tank | **tested** — phase C: 4 operations = 16 damage = 8 souls, `souls == 2 × operations` |
| the lens gate (blue only, none without) | **tested** — phases B and D, the differential above |
| the 1,350 cap | **tested** — `the_tank_stops_at_1350` (unit) |
| the comparator output | **blocked(`analog-output-needs-block-entity-read`)** — formula unit-tested |
| pipe network 4 a pipe, 4-pull from the machine | **tested** — phase E: 50 → 46 with one pipe (4/4), 46 → 42 with two (8/8); the walk is set-based, bounded and once a tick (`a_two_hundred_pipe_line_is_walked_once_a_tick`, `a_network_walk_stops_at_the_budget`, `a_cached_component_keeps_the_ticks_transfer`) |
| the pipe's shape and joins | **carried** — `chorus_plant` template + `connects_to` (not asserted in game; the state string shows `east=true` next to its neighbour) |
| surge placement (`state_for_placement` → `facing` from the clicked face) | **tested** — phase F: a top-face placement reads `facing=up` |
| surge spends one soul for 300 ticks | **tested** — phase G: network 3 → 2, `wait` 293 → 0 over 1,200 ticks |
| surge random-ticks the block it is mounted on | **tested** — phase G: wheat age 7 with `random_tick_speed 0`, control 0 (1,200-tick window, so a 3 % roll cannot miss) |
| surge disabled while powered | **tested** — phase H: `powered true`, `wait` frozen, then false again |
| the three recipes | **tested** — phase K: ids and counts (8 pipes), ingredients consumed, grid empty; a vanilla stand-in grid crafts nothing |
| loot tables | **tested** — phase L: each block breaks in survival and drops itself |
| tags (`cant_accelerate`, `c:tick_acceleration_disallowed`, `mineable/pickaxe`, `soul_connects`, `c:stones`) | **tested** as they are consumed (the surge's own block is in both cant-accelerate tags, the recipes use `c:stones`, the pickaxe tag is what the survival break needs) |
| the efficiency augment: seating by click, `canAcceptAugment` (speed refused, a second efficiency refused), the bar `floor(20 × eff)` after the first operation, the take-back | **tested** — phases D2–D3: `0.80` seated and the addon gone from the hand, `20` → `16`, 20 souls in a 160-tick window where the plain bar fits 8 operations; a speed addon still `held 1` and the efficiency unchanged; the unit tests pin `bar_max` (0.9 → 18, 0.8 → 16) and the first-bar-stays-20 rule |
| the machine's contents travelling on its item (tank, bar, lens, augment; pipe souls; surge ticks) | **tested** — phase P: the drop's `minecraft:block_entity_data` names all of them, placing it back restores `souls 30`, a 16-tick bar, `lens industrialforegoing:blue_laser_lens` and `augments efficiency 0.80`; an empty machine's item has no payload and places as a fresh 20-tick machine; `SELFTEST=carry` fails on the restored tank |
| a second catalyst lens replacing the seated one | **tested** — phase Q: the machine keeps one lens and the first comes back to the inventory (`held 1`); `SELFTEST=replace` fails on the returned lens |
| `laser_drill_ore/echo_shard` + the `deep_dark` biome tag | **tested** — phase M2: the parent's ore laser base reads `11/11` recipes, resolves `industrialforegoingsouls:laser_drill_ore/echo_shard` only in `minecraft:deep_dark` at depth 0, and one completed bar yields `minecraft:echo_shard x1` for 1,000 FE |
| the recipe advancement | **carried** — the jar's own, loaded without error (the launch's load-error count is 0) |
| waterlogging on the pipe and the surge | **tested** — phase R: a fresh pipe reads `waterlogged=false` and `0/4`; a water-bucket click makes it `true` and leaves the empty bucket in the hand; the waterlogged pipe still pulls the laser base `50 → 46` into `4/4`; an empty-bucket click makes it `false` and puts the water bucket in the inventory; a surge set to `[facing=north,waterlogged=true]` reads both properties back and spends a soul (`3/4`, `wait 292 → 256`); a dry surge reads `false`; `SELFTEST=waterlog` fails on the state. The interface-gated half is `blocked(no-waterlogged-interface)` |
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
- `blocked(no-waterlogged-interface)` — a content block's interface set is its template's, so a block
  of this package cannot answer `instanceof SimpleWaterloggedBlock`: a **flowing fluid** next to the
  pipe does not fill it and a **dispenser** does not either (both ask the interface). The state, the
  engine's water fluid state and the player's bucket are ported; the jar's `getStateForPlacement`
  override (placing the block *into* water) is not in `FINDINGS.md` and was not invented. Issue shape:
  "a content block may declare the waterloggable interface, or the engine can see a declared
  `waterlogged` property as one".
- **The drained bucket goes to the inventory, not the hand** — the pin reverts a water bucket written
  into the held slot from a click (measured 2026-11-23, §1.4); `runtime::bucket_destination` picks the
  first free slot instead. A user notices the bucket appear one slot over.

Closed by 0.2.1: ~~`blocked(missing-parent-recipe-type: industrialforegoing:laser_drill_ore)`~~ — the
parent's 0.2.7 registers the type and this package now carries its own recipe for it.

Closed by 0.2.2: ~~a broken machine does not carry its contents~~ — `block_drops` writes a fresh
`minecraft:block_entity_data` payload on the drop and `on_block_placed_by` merges it back, clamped
(phase P); ~~a second lens voids the first~~ — the new lens swaps with the seated one (phase Q);
~~the parent is undeclared~~ — `[dependencies]` now names it (the refusal log above); ~~the pipe
walk is cubic~~ — set-based, bounded and cached once a tick (§1.3 item 3).

Closed by 0.2.3: ~~waterlogging on the pipe and the surge is not ported~~ — both declare the
`waterlogged` state, the engine derives the water from it, and the block's own click fills and drains
it with a bucket (phase R). The interface-gated half (a flowing fluid, a dispenser) is
`blocked(no-waterlogged-interface)` above.

Still open:

- **The augment slots have no screen.** Upstream's two slots are Titanium's GUI; this port seats and
  returns them by right-click. A player cannot see the slots without reading
  `/industrialforegoingsouls info`.

## 8. The ladder (evidence index)

`ports/_shared/evidence/industrialforegoingsouls/`. Logs with a version suffix are kept as they
were; the shared names (`sdk-test.log`, `sdk-clippy*.log`, `sdk-check*.log`) always hold the **newest**
run, so the 0.2.2 numbers in the first table below are that run's record (its package and check
output survive in the `-review` logs, and its 33-test log was replaced at 0.2.3).

**The 0.2.2 ladder (historical):**

| step | command | result | log |
|---|---|---|---|
| red (review) | `cargo test`, the four new rules against `todo!()` | **29 passed, 4 failed** (`not yet implemented: slice 0.2.2 review`) | `sdk-test-red-review.log` |
| tests | `cargo test` | **33 passed, 0 failed** | `sdk-test.log` |
| fmt + lints | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` | `FMT-CLEAN`, `CLIPPY-CLEAN` | `sdk-clippy-review.log` |
| content | `python3 tools/assemble_content.py --check` | `CONTENT OK: 56 files` | — |
| dependency refusal | one-module `native-build` | `industrialforegoingsouls requires missing mod industrialforegoing (… reason + url)`, exit 1 | `sdk-dependency-refusal.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` ×2, the SDK-generated lock removed from each archive | `CMP-IDENTICAL`, sha256 `7397e359…0e16`, **96,432 B**, 63 entries, no `Cargo.lock` | `sdk-build-review.log` |
| check | `cargo horizon-mod check industrialforegoingsouls.hmod` | `industrialforegoingsouls 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, 96,432 B, exit 0 | `sdk-check-review.log` (prints the sha) |
| fits (pin) | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, exit 0 | `sdk-check-review.log` (prints the sha) |
| fits (other) | `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, exit 0 | `sdk-check-review.log` (prints the sha) |
| package gate | `python3 ports/tools/verify-sdk-package.py industrialforegoingsouls` | **1/1** (33 tests) | — |
| launcher build | `tools/e2e-two-mod.sh build` (stale tag + export removed) | `Compiling industrialforegoing (1 of 2)`, `Compiling industrialforegoingsouls (2 of 2)`, carries 0.2.8 + 0.2.2 | `industrialforegoingsouls/e2e-two-mod-build.log` |
| in-game | `tools/e2e-two-mod.sh run` | `PASS` + `EXIT:0`, phases A–Q + M2 | `industrialforegoingsouls-e2e/e2e-launcher.log` |
| control | the same script on `GAME=$CONTROL_GAME` | `FAIL: the game refused industrialforegoingsouls:soul_laser_base` + `EXIT:1` | `…/e2e-launcher-control.log` |
| selftest ×6 | `SELFTEST=1`, `augment`, `laser`, `grow`, `carry`, `replace` | `FAIL` + `EXIT:1` each | `…/e2e-launcher-selftest{,-augment,-laser,-grow,-carry,-replace}.log` |
| log gate | `python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` | `logs=16 ok=16 missing-exit=0 bad-exit=0` | — |

The **0.2.3** rows of the same table (the waterlogging slice):

| step | command | result | log |
|---|---|---|---|
| red (waterlog rules) | `cargo test`, the four new rules at `todo!()` | **33 passed, 4 failed** (`not yet implemented: slice 0.2.3`) | `sdk-test-red-waterlog.log` |
| red (drain destination) | `cargo test`, `bucket_destination` at `todo!()` | **37 passed, 1 failed** | `sdk-test-red-waterlog-drain.log` |
| tests | `cargo test --locked --offline` | **38 passed, 0 failed** | `sdk-test.log` |
| fmt + lints | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` | `FMT-CLEAN`, `CLIPPY-CLEAN` | `sdk-clippy-waterlog.log` |
| content | `python3 tools/assemble_content.py --check` | `CONTENT OK: 56 files` (two declarations + the surge's 12 variants) | — |
| package | `rm -f Cargo.lock && cargo horizon-mod build` ×2, the SDK-generated lock removed from each archive | `CMP-IDENTICAL`, sha256 `68b1485e…88b8`, **101,380 B**, 63 entries, no `Cargo.lock` | `sdk-build-waterlog.log` |
| check | `cargo horizon-mod check industrialforegoingsouls.hmod` | `industrialforegoingsouls 0.2.3 (native-mod, …)`, 101,380 B, exit 0 | `sdk-check-waterlog.log` |
| fits (pin) | `… fits … --build 0a875da` | `0a875da  made for` | `sdk-check-waterlog.log` |
| fits (other) | `… fits … --build 0b14f8a` | `0b14f8a  fits` | `sdk-check-waterlog.log` |
| package gate | `python3 ports/tools/verify-sdk-package.py industrialforegoingsouls` | **1/1** (38 tests, package = tree) | — |
| launcher build | `tools/e2e-two-mod.sh build` (stale `native-0a875da-f52cbec4dbd6` tag and the export removed first) | `Compiling industrialforegoing (1 of 2)`, **`Compiling industrialforegoingsouls (2 of 2)`**, carries 0.2.10 + 0.2.3 | `industrialforegoingsouls/e2e-two-mod-build.log` |
| in-game | `tools/e2e.sh` on that build (A–R, D2–D3, K–L, P–Q, M2) | `PASS` + `EXIT:0` | `…-e2e/e2e-launcher.log` |
| control + selftest ×7 | `GAME=$CONTROL_GAME`, `SELFTEST=1`, `augment`, `laser`, `grow`, `carry`, `replace`, `waterlog` | `FAIL` + `EXIT:1` each, at the intended assertion | `…-e2e/e2e-launcher-control.log`, `…-selftest{,-augment,-laser,-grow,-carry,-replace,-waterlog}.log` |

## 9. Files

```
Cargo.toml, horizon-mod.toml, LICENSE, README.md, .gitignore
industrialforegoingsouls.hmod     delivered package (101,380 B, sha256 68b1485e…88b8)
SLICE-PLAN-0.2.1.md               the first slice's plan (families, assertions, TDD order)
SLICE-PLAN-0.2.2.md               the carrying slice's plan
SLICE-PLAN-0.2.3.md               the waterlogging slice's plan (the pin's measurements, TDD order)
REVIEW-FIXES-0.2.2.md             the reviewer's four findings and how each is answered
src/lib.rs                        the one impl Hooks + the three Parts
src/runtime.rs                    the pure economy, the network cache and the waterlogging rules, 38 tests
assets/industrialforegoingsouls/  blockstates, models, item definitions, block textures, lang
data/industrialforegoingsouls/    block declarations, creative tab, loot, recipes, tags, advancements
data/c/…, data/minecraft/tags/…   this port's declarations of the convention tags it needs
tools/assemble_content.py         jar -> assets/data, re-spelled for 26.3 (`--check` proves it)
tools/craft_recipe.py             the three recipes + the stand-in control, in a real table
tools/parse_info.py               `/industrialforegoingsouls info` -> shell fields
tools/e2e.sh                      the in-game scenario (A–R, D2–D3 the augment, P the carried contents, Q the lens swap, R the waterlogging, M2 the parent laser)
tools/parse_laser.py              the parent machine's `/industrialforegoing info` parser (its own, copied)
tools/e2e-two-mod.sh              builds the two-mod game (this port + its parent) and runs it
notes.md, FINDINGS.md, VALIDATION.md
```
