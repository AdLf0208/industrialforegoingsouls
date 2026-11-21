# Findings — Industrial Foregoing: Souls 1.21.1-1.10.7

Behaviour extracted from the upstream repository
(`InnovativeOnlineIndustries/Industrial-Foregoing-Souls`, commit
`bf736d2f1140da25747ab97d8c160b44d8a0bb7f`, 2026-01-10, shallow clone, deleted after these
findings) and from the jar the pack installs
(`industrial-foregoing-souls-1.21.1-1.10.7.jar`, sha256 `f4fb3640…67ec`, kept in
`ports/_shared/licence-cache/` as the licence and provenance evidence). No Java was translated: each
paragraph below is behaviour, and the Rust is written from it.

## The soul capability

`capabilities/ISoulHandler.java` is the mod's own handler interface: `getSoulTanks`,
`getSoulInTank`, `getTankCapacity`, `fill(amount, action)`, `drain(maxDrain, action)` with
`Action.EXECUTE | SIMULATE`. `SoulCapabilities.BLOCK` is a NeoForge `BlockCapability` created by
`BlockCapability.createSided(industrialforegoingsouls:soul, ISoulHandler.class)`.

**`SLBSoulCap` is the only implementation, and `SoulLaserBaseBlockEntity` is the only block that
registers it.** `fill` returns 0 always; `drain(maxDrain, EXECUTE)` calls `blockEntity.useSoul`,
which returns `min(old, maxDrain)` and never goes below 0. So the laser base is the mod's only soul
*source*, and a pipe can only ever pull from it.

## `SoulLaserBaseBlockEntity`

- No energy (`EnergyStorageComponent<>(0, 4, 10)` and `setShowEnergy(false)`).
- A `ProgressBarComponent` of `MAX_PROGRESS = 20`, direction `ARROW_RIGHT`, `setProgressIncrease(0)`,
  `setCanReset(true)`, `setOnFinishWork(this::onWork)`; its max is rescaled with the efficiency
  augment (`floor(20 × efficiency)`) after every operation.
- One `SidedInventoryComponent` named `lens`: one slot, limit 1, all six faces `FaceMode.NONE`, no
  input filter in the shipped build (the `LaserLensItem` filter is commented out).
- `onWork()`: the gate is
  `!catalyst.getStackInSlot(0).isEmpty() && catalyst.getStackInSlot(0).getItem().equals(ModuleCore.LASER_LENS[11].get()) && soulAmount < 1350`.
  Index 11 of `LASER_LENS` is `DyeColor.BLUE`'s ordinal, i.e. `industrialforegoing:blue_laser_lens`.
  Then the box `Shapes.box(-1, 0, -1, 2, 3, 2).move(x, y - 1, z)` (so `[x-1, y-1, z-1]` to
  `[x+2, y+2, z+2]`) is searched for a `LivingEntity` whose type is `minecraft:warden` or in
  `c:wardens`; the first is hurt with `damageSources().generic()` for `DAMAGE_PER_OPERATION = 4` when
  `getHealth() > 4 || KILL_WARDEN` (false in the pack), and `2` souls are stored, clamped to 1,350.
  Note the health gate is **strict**: a warden at exactly 4 is never hurt, which is what keeps the
  mod's one source alive.
- `clientTick` plays `SCULK_SOUL` particles and a sonic-boom animation on wardens in the box (client
  only; not ported).
- `getAnalogOutputSignal` = `floor(15 × souls / 1350)`; `hasAnalogOutputSignal` is true.
- `useSoul(n)` = `min(old, n)`.
- `canAcceptAugment` refuses speed; efficiency is accepted (the bar shortens).

## `SoulPipeBlockEntity` / `SoulNetwork`

- `serverTick`, for each of the six directions: `level.getCapability(SoulCapabilities.BLOCK, pos.relative(dir), dir.getOpposite())`.
  With a handler: `simulatedFill = fill(4, SIMULATE)`; `filled = fill(network.drainSouls(simulatedFill), EXECUTE)`;
  **if `filled == 0`**, `simulatedDrain = drain(4, SIMULATE)` then `drain(network.addSouls(simulatedDrain), EXECUTE)`.
  So push first, and only pull when the push moved nothing — and against this mod's own handler the
  push always moves nothing (its `fill` is 0).
- `SoulNetwork` extends Titanium's `Network`: `getMaxSouls() = graph.getElements().size() × SOUL_AMOUNT_PER_PIPE (4)`,
  `addSouls` clamps, `drainSouls` floors at 0, `useSoul(level)` takes exactly one.
- The network's soul count lives on the network object and is saved with it (`writeToNbt` →
  `soulAmount`).
- `SoulPipeBlock` has a three-value enum property per side (`none|block|pipe`, `PipeState`), set on
  placement and in `neighborChanged` from the neighbour's block (`SoulPipeBlock`/`SoulSurgeBlock` →
  `pipe`, a network connection or a soul handler → `block`), and is `SimpleWaterloggedBlock`.

## `SoulSurgeBlockEntity`

- `serverTick` runs only while the block state's `ENABLED` is true. `ENABLED` is set at placement
  from `!hasNeighborSignal` and kept equal to `!hasNeighborSignal` in `neighborChanged`.
- If the network is present: when `tickingTime <= 0` and the network has souls, take one and set
  `tickingTime = SOUL_TIME (300)`. While `tickingTime > 0`, look at
  `pos.relative(facing.getOpposite())` — **the block the surge is mounted on**:
  - a block entity there with a ticker: run its ticker `ACCELERATION_TICK (4)` times, `--tickingTime`;
  - otherwise, if `random.nextDouble() < RANDOM_TICK_ACCELERATION_CHANCE (0.03)`, run the block's
    `randomTick` `BLOCK_ACCELERATION_TICK (4)` times, `--tickingTime`;
  - if the block is air: every `LivingEntity` in the 1×1×1 box that is not in
    `SoulTags.Entities.CANT_ACCELERATE` is ticked `ENTITIES_ACCELERATION_TICK (4)` times and
    `--tickingTime` once per entity.
  The block-entity and air branches are skipped for a block in `SoulTags.Blocks.CANT_ACCELERATE`
  (`industrialforegoingsouls:cant_accelerate`) or `FORGE_CANT_ACCELERATE`
  (`c:tick_acceleration_disallowed`) — both tags list the surge itself.
- A soul's 300 ticks are only spent where something was accelerated: facing empty air with no
  entity, `tickingTime` never falls.
- `SoulSurgeBlock`: `Properties.ofFullCopy(IRON_BLOCK).forceSolidOn()`, six-way `FACING_ALL` from
  `getClickedFace`, `ENABLED`, four connection booleans plus `TOP`, `WATERLOGGED`; shapes are
  hand-built boxes; `neighborChanged` recomputes the connections and the enabled flag and
  `setBlockAndUpdate`s when they changed.

## Tags and data

- `industrialforegoingsouls:cant_accelerate` and `c:tick_acceleration_disallowed`: the soul surge.
- `minecraft:mineable/pickaxe`: all three blocks.
- `industrialforegoingsouls:deep_dark` (biome): `minecraft:deep_dark`, used only by the laser-drill
  recipe.
- Three shaped recipes (`ABA/CDC/GSG` laser base, `SSS/CDC/SSS` → 8 pipes, `ABA/CBC/CPC` surge) using
  `#c:plastics`, `#c:gears/diamond`, `#industrialforegoing:machine_frame/advanced` and `#c:stones`.
- One `industrialforegoing:laser_drill_ore` recipe (echo shard, deep dark) — the **parent's** recipe
  type.

## The parent surface this port binds to

`industrialforegoing` 0.2.4 (the campaign's parent port) ships: the 16 `<colour>_laser_lens` (with
`industrialforegoing:laser_lens` listing them), `plastic` (+ `c:plastics`), the three gears
(+ `c:gears/*`), the four machine frames (+ `industrialforegoing:machine_frame/*` as block and item
tags), and the Water Condensator. Verified before scaffolding:

```
$ cat ports/industrialforegoing/data/industrialforegoing/horizon/item/blue_laser_lens.json
{"max_stack_size": 1, "template": "minecraft:paper"}
$ cat ports/industrialforegoing/data/industrialforegoing/tags/item/machine_frame/advanced.json
{"values": ["industrialforegoing:machine_frame_advanced"]}
```

## The licence

`META-INF/neoforge.mods.toml` declares `license = "MIT"`; `audit_licences.py --mod
industrialforegoingsouls --fetch` reads `declared MIT / effective MIT`. The jar also declares
`industrialforegoing` as a `required` dependency, which is why this port refuses to load without it.
