# `industrialforegoingsouls` — Industrial Foregoing: Souls 0.2.5

The three blocks that are one soul economy, ported from **Industrial Foregoing: Souls 1.21.1-1.10.7**
into Rust for Horizon 26.3 at the campaign pin.

| | |
|---|---|
| class · tier | **content · machines** (`ports/ledger.csv`) |
| licence | **MIT**, read from the jar's `META-INF/neoforge.mods.toml` (`audit_licences.py --mod industrialforegoingsouls --fetch` → `declared MIT / effective MIT`), rank **0** |
| approach (`horizon/docs/modpacks.md`) | **agent port** of the behaviour, with the jar's content carried (models, blockstates, textures, recipes, loot, tags) |
| pin | Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`, horizon-mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| upstream | `InnovativeOnlineIndustries/Industrial-Foregoing-Souls`, commit `bf736d2f1140da25747ab97d8c160b44d8a0bb7f` (2026-01-10, the jar's `1.10.7`); the jar is `ports/_shared/licence-cache/industrial-foregoing-souls-1.21.1-1.10.7.jar`, sha256 `f4fb3640…67ec` |
| package | `industrialforegoingsouls.hmod`, **121,782 B**, sha256 `e73258cc4aef8ff5e5e4e46d0f78b22b64c70a457dcf93a1932e0878d4474157`, 65 entries, no packed `Cargo.lock` |
| registry | **first publication**, open on PR #1605; that PR publishes **`0.2.2`** (0.2.0/0.2.1 were dropped at the reviewer's rule — neither carried the declared dependency or the MIT notice). This is `0.2.5` appended to #1605, title `Add industrialforegoingsouls 0.2.5` (the id is not on `main`) |

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

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

### 1.5 The 0.2.4 slice: the laser base's menu and screen

The gap this closes, verbatim from §7 as 0.2.3 left it:

> - **The augment slots have no screen.** Upstream's two slots are Titanium's GUI; this port seats
>   and returns them by right-click. A player cannot see the slots without reading
>   `/industrialforegoingsouls info`.

Upstream draws the machine through the **parent's** Titanium screen
(`SoulLaserBaseScreen`: one catalyst slot, two augment slots, a progress arrow and an energy-less
tank), and the jar ships **no** GUI texture of its own (`unzip -l` over the jar's `textures/` lists
`block/` only), so the screen here is drawn on the engine's plain panel, like `charginggadgets`'s.

What the pin can express, measured before writing code:

| need | at `0a875da` | evidence |
|---|---|---|
| a menu type a content mod declares | `data/<id>/horizon/menu/<name>.json` — `slots` 1..256, `data_slots` 0..256, `results` | `horizon/docs/mods.md` §"Menu types"; `ports/_shared/sdk/0a875da/` (the SDK this package compiles against) |
| the block's container and its menu | `Registrar::block_container(&BlockKey, ModContainer { slots, faces, inputs, outputs, menu })` | `ports/_shared/sdk/0a875da/crates/horizon-mod-vocab/src/parts/default_capabilities.rs:30-37`, `.../src/item/containers.rs:209-215` |
| the client's screen for it | `BlockEntityPart::screen(&dyn Host, &mut ScreenEvent) -> Option<Screen>` + `screen_draw(&dyn ClientHost, &mut ScreenDrawEvent)` (`data`, `origin`, `draw`) | `.../generated/mod_api_generated.rs:8951` and `:8960`; `type:Screen` in `surface.json` |
| the menu's data slots | the block entity's `data`: `event.entity.data` in the tick, `BlockEntityRef::data_slots`/`set_data_slot(s)` outside it, 16-bit on the wire | `.../generated/mod_api_generated.rs` (`block_entity_tick`); `.../generated/mod_api_generated_host.rs:71-100` |
| slot rules the mod decides (API 130) | `MenuPart::menu_slot_rules` + `Registrar::menu_part(menu, part)` | `.../src/hooks/menu_slot_rules.rs`; `.../src/parts/menus.rs:33-37` |
| opening it from the block's use | `PlayerRef::open_menu(&mut ServerHost, BlockEntityRef) -> Result<bool, BlockEntityError>` | `.../generated/mod_api_generated_host.rs:4378` |

**The engine's own click path does not open a mod container's content menu at this pin.**
`bibliocraft` measured it on its own package (a vanilla barrel opened 27 slots from the same click,
the mod block opened none) and records the route both it and `rftoolsstorage` took: the mod asks for
its own menu, `player.open_menu(server, entity)`, from `use_without_item`. That also means no
template change is needed (their blocks are `minecraft:end_rod` and
`minecraft:white_glazed_terracotta`), which is why the laser base keeps `minecraft:iron_block`.

**The shape:**

- **The container is the truth.** Three slots, upstream's own order: the catalyst (`lens`, one item)
  at 0 and Titanium's two augment slots at 1 and 2 (`AUGMENT_SLOTS`). The tick reads the lens and
  the efficiency augment from the slots; a click with the lens/augment seats it in its slot
  (`use_item_on`, unchanged in behaviour, now a real slot); a plain click with anything else — and an
  empty hand — opens the menu (`use_without_item` → `PlayerRef::open_menu`).
- **The menu** is `industrialforegoingsouls:soul_laser_base` (the content's own type, 3 slots, 3
  data slots), with no automation: upstream's catalyst is `FaceMode.NONE` on all six faces and
  Titanium's augment inventory is the GUI's, so `faces`/`inputs`/`outputs` are empty and a hopper
  reaches nothing (0.2.0–0.2.3 had no item capability at all).
- **The slot rules** (`menu_slot_rules`) make the menu agree with `canAcceptAugment`: the catalyst
  takes only `industrialforegoing:blue_laser_lens`, an augment slot only an efficiency addon and
  only while none is seated (speed and a second efficiency are refused and the stack stays on the
  cursor), every slot holds one item.
- **The screen** (`native_screen`): the two augment slots either side of the catalyst under a
  176 × 166 panel, the player's inventory beneath; `screen_draw` prints the tank (`souls / 1350`)
  and the bar (`progress / bar`) from the data slots.
- **The data slots** are `[souls, progress, bar]`, written by the tick (`runtime::menu_data`, 16-bit
  clamped). `ctl` does not expose a menu's data slots (its command table lists slots, carried and
  station detail only; `botanypots` records the same limitation), so `/industrialforegoingsouls info`
  now prints the block entity's `data` array as `menu [souls, progress, bar]` — the very values the
  menu syncs — and the e2e asserts them there.
- **The carry had to move to a stash.** A mod container is spilled by the engine on break
  (`PreRemove`'s `Contents`), so the lens and the addon would have dropped on the ground *and* ridden
  the item. `on_destroyed_by_player` (a harvesting break only) now moves the three slots into a
  short-lived stash keyed by dimension and position and empties them in one transaction; `block_drops`
  takes that stash and puts the stacks in the payload as `carried` — each a stack's own SNBT with its
  `Slot`, so an augment's `titanium:augments` component (a `/give` override included) travels too.
  Placing it back merges the numbers and seats the stacks in one transaction. The 0.2.2 payload's
  `lens`/`augments`/`augment_item` keys are gone; a save or item from 0.2.x is still adopted into the
  slots on the machine's first tick (`runtime::adopted_items`, the `slots` flag).
- **A like-for-like click takes the slot's stack.** Clicking the *same* addon item onto a seating one
  goes through the pin's same-item top-up path, which ignores `may_place` (measured: the slot emptied
  and both addons ended in the player's hands). The refusal the brief asks for is therefore proven
  with the tier-1 addon onto the seated tier-2 one: different items, `may_place` false, nothing moves.
  Nothing is voided either way.

### 1.6 The 0.2.5 slice: the jar's config values as a real config file

The gap this closes, verbatim from §2 as 0.2.4 left it:

> config (`SOUL_STORAGE_AMOUNT 1350`, `MAX_PROGRESS 20`, `SOULS_PER_OPERATION 2`,
> `DAMAGE_PER_OPERATION 4`, `SOUL_TIME 300`, `BLOCK_ACCELERATION_TICK 4`,
> `RANDOM_TICK_ACCELERATION_CHANCE 0.03`, `SOUL_AMOUNT_PER_PIPE 4`) | `config/*.java` | the values
> are `runtime.rs` constants; **no config file** (a recorded deviation)

and §3 item 7 ("No config file. Every number is a `runtime.rs` constant; a server owner cannot retune
the economy without a new build"). The eight names in the gap list were this port's own notes; the
jar's surface is **twelve** keys in two Titanium config files, re-read here from the upstream source
at `bf736d2f` (`config/ConfigSoulLaserBase.java`, `config/ConfigSoulSurge.java`,
`config/IFSoulsMachines.java`, `config/IFSoulsClient.java`):

| key | jar default | jar range | read by (jar) | this port |
|---|---|---|---|---|
| `SOUL_STORAGE_AMOUNT` | `1350` | `min 1` | `SLBSoulCap.getTankCapacity`, `onWork`'s gate and clamp, the comparator, the parent's Sculk screen | `soul_storage` — tank, clamp, comparator, screen label |
| `MAX_PROGRESS` | `20` | `min 1` | the `ProgressBarComponent` and `onWork`'s rescale | `max_progress` — the bar and `bar_max` |
| `KILL_WARDEN` | `false` | boolean | `onWork`'s `health > damage \|\| KILL_WARDEN` | `kill_warden` — the same gate |
| `DAMAGE_PER_OPERATION` | `4` | `min 0` | `onWork`'s hurt and its strict health gate | `damage_per_operation` |
| `SOULS_PER_OPERATION` | `2` | `min 1` | `onWork`'s clamp | `souls_per_operation` |
| `SOUL_TIME` | `300` (`15 * 20`) | `min 1` | `SoulSurgeBlockEntity.serverTick` | `soul_time` |
| `ACCELERATION_TICK` | `4` | `min 0` | the surge's block-entity branch | accepted, unused: `blocked(no-tick-another-block-entity)` |
| `ENTITIES_ACCELERATION_TICK` | `4` | `min 0` | the surge's entity branch | accepted, unused: `blocked(no-entity-tick)` |
| `BLOCK_ACCELERATION_TICK` | `4` | `min 0` | the surge's random-tick branch | `block_acceleration_tick` |
| `RANDOM_TICK_ACCELERATION_CHANCE` | `0.03` | `0..=1` | the same branch | `random_tick_acceleration_chance` |
| `SOUL_AMOUNT_PER_PIPE` | `4` | `min 0` | `SoulNetwork.getMaxSouls` | `soul_per_pipe` |
| `SOUL_PIPES_PARTICLES` | `0.1` | `0..=1` | `SoulPipeBlock`'s client particles | accepted, unused: the client half is not ported |

- **The file is `config/industrialforegoingsouls/config.json`**, a folder of this mod's own, which is
  what the pin lets a mod claim (`ConfigPath::owned_by`). The jar's own file names are Titanium's
  (`industrialforegoing-souls-machines`), which a mod id of `industrialforegoingsouls` cannot own; the
  keys are the settings lower-cased, and the jar's own spellings (`soul_storage_amount`,
  `soul_amount_per_pipe`, `soul_pipes_particles`) are accepted too.
- **It is declared, not read by hand.** `register_configs` pushes it as a `ConfigKind::Common` file —
  the jar's own kind (one file per instance, read once at start) — with the default template, and the
  game writes that template when the file is missing, so a server owner finds all twelve keys instead
  of guessing. `config_loading` hands the text in the Config phase (before any world, therefore before
  any block entity ticks) and `config_reloading` replaces it when the file changes on disk; the engine
  does the polling. A tick only copies the `Copy` value out of one `RwLock`, so **no file I/O happens
  in a tick** — the defect the brief named.
- **JSON, parsed in the pure module.** The pin's convention is TOML, but a parser crate is a
  dependency the pin refuses (§3's "a port cannot add a crate"), and a flat object of twelve scalars
  needs none: `runtime::parse_config` is a ~40-line character walker over `"key": value` pairs,
  unit-tested against an empty file, a half-written one, a byte-order mark, TOML's `key = value`
  shape, a word where a number belongs, a fraction on an integer key, non-ASCII noise, unknown keys
  and both spellings. It never fails: anything it cannot read leaves the jar's default in place, and
  every value it does read goes through `Config::clamped` (the jar's `min`/`max`).
- **The clamp is the jar's.** `soul_storage`, `max_progress`, `souls_per_operation` and `soul_time`
  are at least 1 (a tank, a bar, a yield or a soul of zero would be a machine that never runs), the
  `min = 0` keys floor at 0 (the jar declares no maximum, and this port adds none), and the two
  chances are clamped to `0..=1`. A hand-built `Config` is clamped the same way by `clamped()`.
- **The value in force is threaded, not global.** `runtime::Settings` (the type is called `Settings`,
  not `Config`, for a measured reason below) is a parameter of every rule that reads a number —
  `LaserBase::from_saved`/`tick`/`analog_output`, `bar_max`, `Network::{new,capacity,add,set}`,
  `pipe_pull`, `Surge::tick`, `Walked`/`NetworkCache` — so `runtime.rs` stays pure and the config is a
  value a test can tune.
- **A name collision, measured.** The first build failed `fits --build 0b14f8a` with *"the type
  `Config` can no longer be built with a literal, and `src/runtime.rs:86` builds it"*: the `fits`
  scanner attributes a struct literal whose name matches an API type to that type
  (`mod_api::Config` is `non_exhaustive` and not constructible), and this port's runtime type was
  called `Config`. Renaming it to `Settings` (no such API type) made both fits lines green; the
  ported code did not change. `sdk-check-0.2.5.log` / `sdk-fits-0.2.5.log` are that run.

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
| the efficiency augment (`AugmentTypes.EFFICIENCY`, two `titanium:augments` slots) | `SoulLaserBaseBlockEntity.canAcceptAugment`, `onWork` | **ported and tested** (phases D2–D3, and in the menu at S) |
| the machine's GUI (Titanium's screen: catalysts, augments, progress) | `SoulLaserBaseScreen` is the **parent's**; the jar ships no GUI texture | **ported and tested** (phase S): a menu type of this package, 3 slots + 3 data slots, and a screen that draws the tank and the bar |
| recipe advancements ×3 | `advancement/recipes/misc/*.json` | **ported** (the jar's, `neoforge:conditions` dropped) |
| `SculkSoulTankScreenAddon` (client) | `client/SculkSoulTankScreenAddon.java` | **not ported** — it draws on the parent's Sculk Soul Tank GUI, which the parent port does not ship; there is no screen to add to |
| comparator output (`getAnalogOutputSignal`) | `block/SoulLaserBaseBlock.java:31-39` | **blocked(`analog-output-needs-block-entity-read`)** — the `analog_output` hook is given `&dyn LevelHost`, which has no way to resolve a block entity by position. Formula and test (`LaserBase::analog_output`) are in `runtime.rs`, ready for the hook |
| the surge's block-entity branch (×4 ticks of a machine in front) | `SoulSurgeBlockEntity.java:60-70` | **blocked(`no-tick-another-block-entity`)** — `block_entity_tick` ticks the owning entity only |
| the surge's entity branch (×4 ticks of `LivingEntity`) | `SoulSurgeBlockEntity.java:78-90` | **blocked(`no-entity-tick`)** — no helper ticks an entity |
| config: twelve settings in two Titanium files (`ConfigSoulLaserBase`, `ConfigSoulSurge`, `IFSoulsMachines`, `IFSoulsClient`) | `config/*.java` | **ported and tested** (phase T): the game writes the twelve-key template when the file is missing, and a tuned file read before the launch changes the tank (500), the operation (5 souls) and leaves the keys it does not name at the default |
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
3. **The catalyst slot is a real slot, seated by the click or the menu.** A right-click with the lens
   puts it in slot 0 (taking one out of the hand), and the menu's slot 300 shows and changes it. A
   second lens is a swap, as the GUI's cursor would be: the old lens goes back into the hand that
   empties (or the first free slot), and a full inventory refuses the click rather than voiding it
   (`runtime::replacement_slot`). Upstream's `SidedInventoryComponent` is `FaceMode.NONE`, and this
   port's container declares no reachable face either, so a hopper touches nothing.
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
7. **The config file is JSON in a folder of this mod's own, not the jar's TOML beside it.** The
   numbers are the jar's twelve settings (§1.6) and the port reads them from
   `config/industrialforegoingsouls/config.json`. Two reasons: the pin refuses a parser crate, so
   TOML would mean a hand-written TOML parser for no gain over a flat object; and the jar's file name
   belongs to another mod id, which the pin's ownership rule would not let this one read
   (`ConfigPath::owned_by`). The jar's own key spellings are accepted as aliases, so a settings table
   copied from a NeoForge install still lands.
8. **The augment slots are the machine's own two slots of a GUI of its own.** Titanium's two-slot
   augment inventory is the container's slots 1 and 2 (the addon item itself, with its
   `titanium:augments` component); the machine's `use_item_on` seats one and takes one out of the
   hand, the menu shows and changes them, a speed addon is refused and stays in the hand, and a
   second efficiency addon is refused by `menu_slot_rules` and by the click alike
   (`canAcceptAugment`). The screen is not Titanium's: `SoulLaserBaseScreen` lives in the **parent**
   and the jar carries no GUI texture, so this package declares its own menu type and draws the tank
   and the bar on the engine's plain panel (`native_screen`/`screen_draw`). Upstream's energy-less
   `ProgressBarComponent` draws a right-facing arrow; this screen prints `progress / bar` instead
   (no GUI texture to draw an arrow with), a recorded deviation.
   **Measured platform behaviour, not assumed:** the engine also calls `use_without_item` after an
   item's `use_item_on` returns pass, with the stack still in the hand; the menu now opens on that
   fall-through (the GUI is what a machine with nothing to do with the item shows), which is why
   every click that can be refused closes the menu before the next phase aims at the block.
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

### 4.4 The 0.2.4 slice's red → green (the menu and the screen)

The eight new rules were written first, against `todo!()` stubs, with the 38 carried tests running
unchanged; only the new family failed:

```
$ cargo test
test runtime::tests::a_data_slot_stops_at_the_sixteen_bit_top ... FAILED
test runtime::tests::a_legacy_save_adopts_its_lens_and_augments_into_the_slots ... FAILED
test runtime::tests::a_second_efficiency_addon_is_refused_by_either_augment_slot ... FAILED
test runtime::tests::a_speed_addon_is_refused ... FAILED
test runtime::tests::an_efficiency_addon_goes_into_the_first_empty_augment_slot ... FAILED
test runtime::tests::the_catalyst_slot_takes_only_the_blue_lens ... FAILED
test runtime::tests::the_menu_syncs_the_tank_the_progress_and_the_bar ... FAILED
test runtime::tests::the_player_inventory_slots_are_not_the_machines_to_refuse ... FAILED
thread 'runtime::tests::the_menu_syncs_the_tank_the_progress_and_the_bar' panicked at src/runtime.rs:551:5:
not yet implemented: slice 0.2.4: the menu's three data slots are the tank and the bar
test result: FAILED. 38 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out
```

(`ports/_shared/evidence/industrialforegoingsouls/sdk-test-red-0.2.4.log`.) Green:

```
$ cargo test
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

(`sdk-test.log`.) One of my own expectations was wrong against the rule and was corrected in the
test: a `LaserBase` read from a save is already clamped to the 1,350 tank, so the 16-bit data-slot
clamp cannot be reached through `from_saved` — the test builds the `LaserBase` literal the clamp is
for.

The content red was the carry tool's own: `python3 tools/assemble_content.py --check` answered
`differs or missing: data/industrialforegoingsouls/horizon/menu/soul_laser_base.json`, and
`CONTENT OK: 57 files` after the declaration was added.

The in-game legs found four real defects this slice, each traced rather than retried:

1. **`ctl` does not answer `screen=inventory` for a container.** The first run waited on the wrong
   name and failed at phase D with `no container menu opened at 194,101,0 (screen=container)`: the
   menu *had* opened (`ctl state` calls every container screen `container`).
2. **A held item seats instead of opening the menu.** Phase S's lens went into the catalyst slot
   with the hand holding it, so the second `open_menu_at` found no menu; `hand_off` (select an empty
   hotbar slot) is the GUI's own gesture, and the click that follows seats nothing.
3. **`peaceful` removes the warden.** R sets the difficulty peaceful for the breaking phases, and
   the harvest's warden does not survive it; phase S sets it back to normal before summoning one.
4. **A mod container spills on break.** Phase P found the lens on the ground *and* on the item:
   `on_destroyed_by_player` + the stash (see §1.5) is the fix, and the payload's `carried` list is
   the record (the sixth run's log line "the lens the break did not spill is 1, expected 0" is that
   red).

### 4.5 The 0.2.5 slice's red → green (the config file)

Two reds, because the slice has two halves: the file's parsing, then the rules reading it.

**The parser first**, against `todo!()` stubs, with the 46 carried tests running unchanged:

```
$ cargo test
test runtime::tests::no_config_file_is_the_jars_defaults ... FAILED
test runtime::tests::the_config_file_reads_every_key ... FAILED
test runtime::tests::the_minimums_the_jar_declares_are_enforced ... FAILED
test runtime::tests::a_value_outside_its_range_is_clamped_not_refused ... FAILED
test runtime::tests::a_partial_or_broken_file_keeps_the_defaults ... FAILED
test runtime::tests::unknown_keys_are_ignored_and_both_spellings_are_read ... FAILED
test runtime::tests::the_written_file_parses_back_and_names_every_key ... FAILED
thread 'runtime::tests::no_config_file_is_the_jars_defaults' panicked at src/runtime.rs:152:5:
not yet implemented: slice 0.2.5: parse the config file
test result: FAILED. 46 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out
```

(`ports/_shared/evidence/industrialforegoingsouls/sdk-test-red-config.log`.)

**Then the rules**, with `&Settings` threaded through the signatures and every body still reading the
constants, so the five tuned tests were the only failures:

```
$ cargo test
test runtime::tests::a_tuned_bar_damage_and_warden_gate_change_the_operation ... FAILED
test runtime::tests::a_tuned_operation_stores_its_configured_souls ... FAILED
test runtime::tests::a_tuned_pipe_holds_its_configured_souls ... FAILED
test runtime::tests::a_tuned_surge_spends_its_configured_time_and_roll ... FAILED
test runtime::tests::a_tuned_tank_caps_what_the_machine_stores ... FAILED
thread 'runtime::tests::a_tuned_operation_stores_its_configured_souls' panicked at src/runtime.rs:1822:9:
assertion `left == right` failed: SOULS_PER_OPERATION = 5
  left: 2
thread 'runtime::tests::a_tuned_pipe_holds_its_configured_souls' panicked at src/runtime.rs:1881:9:
assertion `left == right` failed: six a pipe
  left: 8
thread 'runtime::tests::a_tuned_surge_spends_its_configured_time_and_roll' panicked at src/runtime.rs:1907:9:
assertion `left == right` failed: SOUL_TIME = 120, less the tick
  left: 299
thread 'runtime::tests::a_tuned_tank_caps_what_the_machine_stores' panicked at src/runtime.rs:1792:9:
assertion `left == right` failed: a saved tank is clamped to the configured capacity
  left: 900
test result: FAILED. 55 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out
```

(`sdk-test-red-0.2.5.log`.) Green:

```
$ cargo test
test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

(`sdk-test.log`.) Two of my own expectations were wrong against upstream and were corrected **in the
tests**, not in the code: the gate is `getHealth() > DAMAGE_PER_OPERATION || KILL_WARDEN`, so a
warden at exactly the damage *is* hurt when the flag is on (the test now sets the flag off for that
assertion); and a soul's first tick both buys the time and spends one tick, so the counter after it
is `soul_time - 1`, not `soul_time`.

Three things the ladder itself taught, all fixed in the tooling rather than worked around:

- **`fits --build 0b14f8a` failed on the type's name**, not on its behaviour: the scanner attributed
  `runtime.rs`'s `Config { … }` literal to `mod_api::Config` (non-constructible). The runtime type is
  `Settings` now; both fits lines are green (§1.6).
- **The remote runner has no `cargo-horizon-mod`**, so the package build, `check` and both `fits`
  runs are `BUILD_LOCAL=1` (§8); the test and clippy steps still forward.
- **A remote `cargo fmt` formats the remote copy**, so the local tree's format is proven by a
  `BUILD_LOCAL=1` run (`FMT-CLEAN` in `sdk-clippy-0.2.5.log`).

The in-game leg found one real defect, and it was the harness's: **phase M2 leaves the world at
`peaceful`**, where the server answers `Monsters cannot be summoned in Peaceful difficulty` and phase
T's warden never existed — the first tuned run failed `80 ticks with a lens and a warden stored no
souls` with `warden none` in every line. Phase T sets the difficulty back to normal (and asserts the
warden is in the box) before it summons one; the second run passed. Everything the config file was
supposed to change was already visible in the failing run — `souls 0/500` and `souls 500/500` — so
the file itself was read correctly the first time.

## 5. In-game evidence (fail-capable)

`ports/_shared/evidence/industrialforegoingsouls-e2e/`, all through `tools/e2e-two-mod.sh` (one game
carrying this port **and** its parent, `industrialforegoing` 0.2.11 when these legs ran). The 0.2.0
legs are kept with a `-0.2.0` suffix and the 0.2.1 ones with `-0.2.1`; the unsuffixed logs are this
0.2.5 run (2026-11-23, 11:58–12:51):

| leg | verdict | what it proves |
|---|---|---|
| `e2e-launcher.log` (0.2.5) | **`PASS` + `EXIT:0`** | the whole scenario, phases A–N, D2–D3, R, S, K–L, P–Q, M2, and this slice's **T** |
| `e2e-launcher-control.log` | `FAIL: the game refused industrialforegoingsouls:soul_laser_base` + `EXIT:1` | the same script on the same Nightly **without** the package |
| `e2e-launcher-selftest.log` | `FAIL: selftest: the soul tank is 0, expected anything but 0` + `EXIT:1` | the lens differential can fail |
| `e2e-launcher-selftest-screen.log` | `FAIL: selftest: the base opened a menu (container), expected none` + `EXIT:1` | **this slice's** menu differential can fail |
| `e2e-launcher-selftest-augment.log` | `FAIL: selftest: the shortened bar gained 22 souls, expected no more than the plain bar's 16` + `EXIT:1` | the augment's rate differential can fail |
| `e2e-launcher-selftest-laser.log` | `FAIL: selftest: the base yielded minecraft:echo_shard x1, expected an empty output slot` + `EXIT:1` | the ore differential can fail |
| `e2e-launcher-selftest-grow.log` | `FAIL: selftest: the control wheat's age is 0, expected it above 0` + `EXIT:1` | the acceleration differential can fail |
| `e2e-launcher-selftest-carry.log` | `FAIL: selftest: the restored tank is 30, expected 0` + `EXIT:1` | the carried contents are real, not a stale machine |
| `e2e-launcher-selftest-replace.log` | `FAIL: selftest: the lens the swap returned is 1, expected 0` + `EXIT:1` | the replaced lens is returned, not voided |
| `e2e-launcher-selftest-waterlog.log` | `FAIL: selftest: the pipe's waterlogged state is true, expected false` + `EXIT:1` | the water the bucket puts in is real, not a readout |
| `e2e-launcher-selftest-config.log` | `FAIL: the tank's capacity, from the config in force is 500, expected 1350` + `EXIT:1` | **this slice's** differential: the tuned file really is in force |

`python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` →
`logs=18 ok=18 missing-exit=0 bad-exit=0` (four 0.2.0, three 0.2.1, and the eleven unsuffixed legs of
this run: the green one, the control and the nine selftests).

**The config transcript**, phase T of this run — the template the game wrote, the tuned file, and what
it changed in the same machine:

```
== T: the config file — the machines run the numbers in it (0.2.5) ==
  the game wrote the template: all twelve keys, soul_storage 1350, souls_per_operation 2
  tuned file: {"soul_storage":500,"souls_per_operation":5}
  Soul Laser Base at [194, 101, 0]: souls 0/500, progress 7/20, lens industrialforegoing:blue_laser_lens, augments efficiency 1.00, menu [0, 7, 20], warden 500.0, t=5098
  seeds 900: souls 500/500, bar 20 (the file named no max_progress)
  the first operation stored 5 souls (the file names souls_per_operation 5)
```

The differential is the whole first launch, which never sees a config file the player wrote: phase C
reads `souls 8/1350` for four operations and `SELFTEST=config` (the same tuned file, the default
expectation) fails at `the tank's capacity, from the config in force is 500, expected 1350`.

The control and the nine selftests ran on the same source tree and manifest as the delivered package,
one `README.md` edit earlier (the README is packed, so the package was rebuilt and the **green** leg
re-run on the final build, `native-0a875da-893f7451dc2f`; the README is documentation, not code, and
the failing legs' verdicts do not depend on it).

**The menu transcript**, phase S of the 0.2.4 leg — the slots, the lens seated through the menu, the
data slots and the differential:

```
  the menu: 3 slots; 301 holds the tier-2 addon D2 seated, 300 and 302 are empty
  Soul Laser Base at [194, 101, 0]: souls 42/1350, progress 5/16, lens industrialforegoing:blue_laser_lens, augments efficiency 0.80, menu [42, 5, 16], warden 500.0, t=2766
  slot 300 now holds the lens the menu seated
  Soul Laser Base at [194, 101, 0]: souls 44/1350, progress 9/16, lens industrialforegoing:blue_laser_lens, augments efficiency 0.80, menu [44, 9, 16], warden 496.0, t=2778
  Soul Laser Base at [194, 101, 0]: souls 50/1350, progress 2/16, lens industrialforegoing:blue_laser_lens, augments efficiency 0.80, menu [50, 2, 16], warden 484.0, t=2833
  the harvest through the menu-seated lens: souls 44 -> 50, the data slots [50, 2, 16]
  after picking the tier-1 addon up: carried industrialforegoing:efficiency_addon_tier_1, 301 industrialforegoing:efficiency_addon_tier_2
  a second efficiency addon is refused by both augment slots and stays on the cursor
  a vanilla barrel beside it opens 27 chest slots where the machine's menu has 3
```

`slot 300` is the menu's catalyst slot (`ctl screen` numbers a container's own slots from 300), `301`
and `302` Titanium's two augment slots. The `menu [...]` field is the block entity's data array — the
three values the menu syncs — and the same line's `souls`/`progress`/`bar` agree with it, because both
are read in one command with no tick between them.

**The carried payload**, phase P of the same leg — the slots ride `carried`, nothing spills:

```
  the drop carries souls 30, progress 11/16, and the stacks [industrialforegoing:blue_laser_lens,industrialforegoing:efficiency_addon_tier_2]
  … {components: {"minecraft:block_entity_data": {carried: [{count: 1, Slot: 0b, id: "industrialforegoing:blue_laser_lens"},
    {components: {"titanium:augments": {Efficiency: 0.8d}}, count: 1, Slot: 1b, id: "industrialforegoing:efficiency_addon_tier_2"}],
    bar: 16, souls: 30, progress: 11, id: "industrialforegoingsouls:soul_laser_base"}}, …}]
```

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
| the machine's contents travelling on its item (tank, bar, lens, augment; pipe souls; surge ticks) | **tested** — phase P: the drop's `minecraft:block_entity_data` names the tank and the bar and its `carried` list the catalyst's lens and the augment slot's addon (`items=` in `parse_carry.py`'s reply), **nothing spills**, placing it back restores `souls 30`, a 16-tick bar, `lens industrialforegoing:blue_laser_lens` and `augments efficiency 0.80`; an empty machine's item has no payload and places as a fresh 20-tick machine; `SELFTEST=carry` fails on the restored tank |
| the laser base's menu (`industrialforegoingsouls:soul_laser_base`) | **tested** — phase S: a plain right-click opens it, `ctl screen` lists **3** container slots (300 catalyst, 301/302 augments) with the tier-2 addon in 301; the lens goes in through slot 300 and the machine harvests (`souls 44 → 50`, warden 500 → 484); a vanilla barrel beside it opens 27 chest slots (the differential); `SELFTEST=screen` asserts the base opens **no** menu and fails against this package |
| the menu's data slots `[souls, progress, bar]` | **tested** — phase S: `/industrialforegoingsouls info` prints the block entity's `data` as `menu [...]`, and after 40 ticks the tank in it grew with the tank the same line reads (`menu [50, 2, 16]`), the bar in it is the bar (`16`), and the progress is inside it. `ctl` has no data-slot field (`botanypots` records the same), so the assertion is the server's own data array — the exact values the menu syncs |
| `menu_slot_rules` (API 130): catalyst takes only the blue lens, an augment slot one efficiency addon | **tested** — phase S: a tier-1 efficiency addon clicked into the occupied 301 and the empty 302 is refused by both (`301` keeps the tier-2, `302` stays empty, the tier-1 stays on the cursor); unit: `the_catalyst_slot_takes_only_the_blue_lens`, `a_second_efficiency_addon_is_refused_by_either_augment_slot`, `a_speed_addon_is_refused` |
| the laser base's menu (`industrialforegoingsouls:soul_laser_base`) | **tested** — phase S: a plain right-click opens it, `ctl screen` lists **3** container slots (300 catalyst, 301/302 augments) with the tier-2 addon in 301; the lens goes in through slot 300 and the machine harvests (`souls 44 → 50`, warden 500 → 484); a vanilla barrel beside it opens 27 chest slots (the differential); `SELFTEST=screen` asserts the base opens **no** menu and fails against this package |
| the menu's data slots `[souls, progress, bar]` | **tested** — phase S: `/industrialforegoingsouls info` prints the block entity's `data` as `menu [...]`, and after 40 ticks the tank in it grew with the tank the same line reads (`menu [50, 2, 16]`), the bar in it is the bar (`16`), and the progress is inside it. `ctl` has no data-slot field (`botanypots` records the same), so the assertion is the server's own data array — the exact values the menu syncs |
| `menu_slot_rules` (API 130): the catalyst takes only the blue lens, an augment slot one efficiency addon | **tested** — phase S: a tier-1 efficiency addon clicked into the occupied 301 and the empty 302 is refused by both (301 keeps the tier-2, 302 stays empty, the tier-1 stays on the cursor); unit: `the_catalyst_slot_takes_only_the_blue_lens`, `a_second_efficiency_addon_is_refused_by_either_augment_slot`, `a_speed_addon_is_refused` |
| the screen drawing the tank and the bar (`native_screen_draw`) | **drawn, not asserted** — with `--offscreen` there is no pixel to read, and `ctl` does not read a screen's text; the values it draws are the data slots the row above proves, and the layout is the `Screen` the hook returns (3 slots, no hidden one) |
| a second catalyst lens replacing the seated one | **tested** — phase Q: the machine keeps one lens and the first comes back to the inventory (`held 1`); `SELFTEST=replace` fails on the returned lens |
| `laser_drill_ore/echo_shard` + the `deep_dark` biome tag | **tested** — phase M2: the parent's ore laser base reads `11/11` recipes, resolves `industrialforegoingsouls:laser_drill_ore/echo_shard` only in `minecraft:deep_dark` at depth 0, and one completed bar yields `minecraft:echo_shard x1` for 1,000 FE |
| the recipe advancement | **carried** — the jar's own, loaded without error (the launch's load-error count is 0) |
| waterlogging on the pipe and the surge | **tested** — phase R: a fresh pipe reads `waterlogged=false` and `0/4`; a water-bucket click makes it `true` and leaves the empty bucket in the hand; the waterlogged pipe still pulls the laser base `50 → 46` into `4/4`; an empty-bucket click makes it `false` and puts the water bucket in the inventory; a surge set to `[facing=north,waterlogged=true]` reads both properties back and spends a soul (`3/4`, `wait 292 → 256`); a dry surge reads `false`; `SELFTEST=waterlog` fails on the state. The interface-gated half is `blocked(no-waterlogged-interface)` |
| Sculk Soul Tank screen addon | **not ported** — parent GUI |
| surge BE / entity branches | **blocked** — `no-tick-another-block-entity`, `no-entity-tick` |
| the config file (`config/industrialforegoingsouls/config.json`) | **tested** — phase T: the game writes the twelve-key template with the jar's defaults (`soul_storage 1350`, `souls_per_operation 2`); a tuned file `{"soul_storage": 500, "souls_per_operation": 5}` written **before** a second launch makes the same machine read `souls 0/500`, clamp a seeded 900 to `500/500`, keep the unnamed `max_progress` at 20, carry 500 in the menu's data slot, and store **5** souls in the first operation, where the untuned first launch stored 2 and capped at 1,350; `SELFTEST=config` asserts 1,350/2 on that tuned file and fails. Unit: `no_config_file_is_the_jars_defaults`, `the_config_file_reads_every_key`, `the_minimums_the_jar_declares_are_enforced`, `a_value_outside_its_range_is_clamped_not_refused`, `a_partial_or_broken_file_keeps_the_defaults`, `unknown_keys_are_ignored_and_both_spellings_are_read`, `the_written_file_parses_back_and_names_every_key`, and the five tuned-rule tests |
| the two acceleration-branch keys (`acceleration_tick`, `entities_acceleration_tick`) and the client particle key | **accepted, not used** — the branches are `blocked(no-tick-another-block-entity)` / `blocked(no-entity-tick)` and the client half is not ported; the keys stay in the file so the surface matches the jar |

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
- **`ctl` cannot read a menu's data slots.** Its command table lists a menu's slots, carried stack and
  station detail only (`horizon/docs/horizon-ctl-commands.md`), and `botanypots` records the same
  limit for its own menu. A mod's data slots are therefore asserted through the mod's own readout
  (`menu [...]`, the block entity's `data`) or by what they make the server do. Issue shape: "a
  control client reads a container menu's data slots (the synced `ContainerData`), so a machine's
  screen readouts are assertable from a headless run".
- **A like-for-like click takes a refused slot's stack.** Clicking the *same* item onto an occupied
  slot whose `may_place` is false goes through the pin's same-item top-up path
  (`menu.rs`'s `pickup_inner`, the `else if clicked.same_item_same_components` branch), which ignores
  the refusal: the slot empties and the two stacks end up on the player's cursor (§1.5). Nothing is
  voided, and a *different* item is refused as the rule says. Issue shape: "a refused placement
  (`may_place == false`) refuses the same-item top-up too".

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

Closed by 0.2.5: ~~no config file~~ — the jar's twelve settings are read from
`config/industrialforegoingsouls/config.json`, which the game writes with the jar's defaults when it
is missing, once at load and again when it changes (§1.6, phase T). The three keys whose behaviour
this pin cannot express (`acceleration_tick`, `entities_acceleration_tick`, `pipes_particles`) are
accepted and kept rather than dropped.

Closed by 0.2.4: ~~the augment slots have no screen~~ — the machine has a container of three slots
and a menu type of its own, opened by a plain right-click, with a screen that draws the tank and the
bar and data slots that carry them (phase S). The container also gave the machine a real inventory,
which moved the carried contents to the stash/`carried` payload described in §1.5.

Still open:

- **The screen has no texture and no arrow.** The parent's Titanium screen and its GUI texture are
  not this package's, and the jar carries none; the tank and the bar are printed as
  `souls / <tank>` and `progress / bar` over the engine's plain panel. A faithful redraw needs the
  parent's texture (a cross-package asset dependency this pin does not express).
- **The screen's tank bar and progress arrow, drawn with `fill`/`quad`, is a named gap and is not
  queued.** With `--offscreen` there is no pixel to read and `ctl` reads no screen text, so a bar
  drawn as geometry would have no strict headless assertion — the numbers it would draw are already
  asserted through the menu's data slots (phase S and phase T). It stays open until the harness can
  read a screen's shapes, not because the drawing is unknown.
- **The screen's labels are literal English.** `screen_draw` prints "catalyst", "augments" and the
  two counters without a translation key, so they are not in `lang/` (the title and "Inventory" are
  the engine's and are).

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

The **0.2.4** rows of the same table (the menu-and-screen slice):

| step | command | result | log |
|---|---|---|---|
| red (menu rules) | `cargo test`, the eight new rules at `todo!()` | **38 passed, 8 failed** (`not yet implemented: slice 0.2.4`) | `sdk-test-red-0.2.4.log` |
| tests | `cargo test --offline` | **46 passed, 0 failed** | `sdk-test.log` |
| fmt + lints | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` | `FMT-CLEAN`, `CLIPPY-EXIT:0` | `sdk-clippy-0.2.4.log` |
| content | `python3 tools/assemble_content.py --check` | red on the missing menu declaration, then `CONTENT OK: 57 files` | — |
| package | `rm -f Cargo.lock && cargo horizon-mod build` ×2, the SDK-generated lock stripped from each archive | `CMP-IDENTICAL`, sha256 `79159d3f…a82b`, **111,572 B**, 64 entries, `PACKED-LOCK: 0` | `sdk-build-0.2.4.log` |
| check | `cargo horizon-mod check industrialforegoingsouls.hmod` | `industrialforegoingsouls 0.2.4 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, 111,572 B, sha256 `79159d3f…a82b`, exit 0 | `sdk-check-0.2.4.log` |
| fits (pin) | `… fits … --build 0a875da` | `0a875da  made for` | `sdk-fits-0.2.4.log` |
| fits (other) | `… fits … --build 0b14f8a` | `0b14f8a  fits` | `sdk-fits-0.2.4.log` |
| package gate | `python3 ports/tools/verify-sdk-package.py industrialforegoingsouls` | **1/1** (46 tests, package = tree) | — |
| launcher build | `tools/e2e-two-mod.sh build` (the three stale `native-0a875da-*` tags carrying this port and the export removed first) | `Compiling industrialforegoing (1 of 2)`, **`Compiling industrialforegoingsouls (2 of 2)`**, `the build carries: industrialforegoing 0.2.10, industrialforegoingsouls 0.2.4` | `industrialforegoingsouls/e2e-two-mod-build-0.2.4.log` |
| in-game | `tools/e2e.sh` on that build (A–N, D2–D3, R, S, K–L, P–Q, M2) | **`PASS` + `EXIT:0`** | `…-e2e/e2e-launcher.log` |
| control + selftest ×8 | `GAME=$CONTROL_GAME`, `SELFTEST=1`, `screen`, `augment`, `laser`, `grow`, `carry`, `replace`, `waterlog` | `FAIL` + `EXIT:1` each, at the intended assertion | `…-e2e/e2e-launcher-control.log`, `…-selftest{,-screen,-augment,-laser,-grow,-carry,-replace,-waterlog}.log` |
| log gate | `python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` | `logs=17 ok=17 missing-exit=0 bad-exit=0` | — |

The **0.2.5** rows of the same table (the config-file slice). The package, `check` and both `fits`
runs are `BUILD_LOCAL=1` because the remote runner has no `cargo-horizon-mod`; `test` and the lints
forwarded, and the local format was proven separately (§4.5):

| step | command | result | log |
|---|---|---|---|
| red (the parser) | `cargo test`, the seven config rules at `todo!()` | **46 passed, 7 failed** (`not yet implemented: slice 0.2.5: parse the config file`) | `sdk-test-red-config.log` |
| red (the rules) | `cargo test`, `&Settings` threaded but the bodies still on the constants | **55 passed, 5 failed** (the five tuned-rule tests) | `sdk-test-red-0.2.5.log` |
| tests | `cargo test --locked --offline` | **60 passed, 0 failed** | `sdk-test.log` |
| fmt + lints | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (locally, after a remote `cargo fmt`) | `FMT-CLEAN`, `CLIPPY-EXIT:0` | `sdk-clippy-0.2.5.log` |
| content | `python3 tools/assemble_content.py --check` | `CONTENT OK: 57 files` (the slice adds no content) | — |
| package | `rm -f Cargo.lock && cargo horizon-mod build` ×2, the SDK-generated lock stripped from each archive | `CMP-IDENTICAL`, sha256 `e73258cc…4157`, **121,782 B**, 65 entries, `PACKED-LOCK: 0` | `sdk-build-0.2.5.log` |
| check | `cargo horizon-mod check industrialforegoingsouls.hmod` | `industrialforegoingsouls 0.2.5 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, 121,782 B, sha256 `e73258cc…4157`, exit 0 | `sdk-check-0.2.5.log` |
| fits (pin) | `… fits … --build 0a875da` | `0a875da  made for` | `sdk-fits-0.2.5.log` |
| fits (other) | `… fits … --build 0b14f8a` | `0b14f8a  fits` (after the `Config` → `Settings` rename, §1.6) | `sdk-fits-0.2.5.log` |
| package gate | `python3 ports/tools/verify-sdk-package.py industrialforegoingsouls` | **1/1** (60 tests, package = tree) | — |
| launcher build | `tools/e2e-two-mod.sh build` | `Compiling industrialforegoing (1 of 2)`, **`Compiling industrialforegoingsouls (2 of 2)`**, `the build carries: industrialforegoing 0.2.11, industrialforegoingsouls 0.2.5` (`native-0a875da-893f7451dc2f`) | `industrialforegoingsouls/e2e-two-mod-build-0.2.5.log` |
| in-game | `tools/e2e.sh` on that build (A–N, D2–D3, R, S, K–L, P–Q, M2, **T**) | **`PASS` + `EXIT:0`** | `…-e2e/e2e-launcher.log` |
| control + selftest ×9 | `GAME=$CONTROL_GAME`, `SELFTEST=1`, `screen`, `augment`, `laser`, `grow`, `carry`, `replace`, `waterlog`, **`config`** | `FAIL` + `EXIT:1` each, at the intended assertion | `…-e2e/e2e-launcher-control.log`, `…-selftest{,-screen,-augment,-laser,-grow,-carry,-replace,-waterlog,-config}.log` |
| log gate | `python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` | `logs=18 ok=18 missing-exit=0 bad-exit=0` | — |

## 9. Files

```
Cargo.toml, horizon-mod.toml, LICENSE, README.md, .gitignore
industrialforegoingsouls.hmod     delivered package (121,782 B, sha256 e73258cc…4157)
SLICE-PLAN-0.2.1.md               the first slice's plan (families, assertions, TDD order)
SLICE-PLAN-0.2.2.md               the carrying slice's plan
SLICE-PLAN-0.2.3.md               the waterlogging slice's plan (the pin's measurements, TDD order)
SLICE-PLAN-0.2.4.md               the menu-and-screen slice's plan (the API check, TDD order)
SLICE-PLAN-0.2.5.md               the config-file slice's plan (the jar's twelve keys, the file, TDD order)
REVIEW-FIXES-0.2.2.md             the reviewer's four findings and how each is answered
src/lib.rs                        the one impl Hooks + the three Parts + the menu, the screen and the break stash
src/config.rs                     config/industrialforegoingsouls/config.json: the value in force, read once
src/runtime.rs                    the pure economy, its Settings, the network cache, the waterlogging and the menu rules, 60 tests
assets/industrialforegoingsouls/  blockstates, models, item definitions, block textures, lang
data/industrialforegoingsouls/    block declarations, the menu type, creative tab, loot, recipes, tags, advancements
data/c/…, data/minecraft/tags/…   this port's declarations of the convention tags it needs
tools/assemble_content.py         jar -> assets/data, re-spelled for 26.3 (`--check` proves it)
tools/craft_recipe.py             the three recipes + the stand-in control, in a real table
tools/parse_info.py               `/industrialforegoingsouls info` -> shell fields (with the menu's data slots)
tools/parse_carry.py              a broken machine's `minecraft:block_entity_data` payload -> its tank and its `carried` stacks
tools/e2e.sh                      the in-game scenario (A–R, D2–D3 the augment, S the menu, P the carried contents, Q the lens swap, R the waterlogging, M2 the parent laser, T the config file)
tools/build-package.sh            the delivered package, built twice and compared, the SDK lock stripped
tools/parse_laser.py              the parent machine's `/industrialforegoing info` parser (its own, copied)
tools/e2e-two-mod.sh              builds the two-mod game (this port + its parent) and runs it
notes.md, FINDINGS.md, VALIDATION.md
```
