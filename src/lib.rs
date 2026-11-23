//! Industrial Foregoing: Souls — the one `impl Hooks` at the crate root.
//!
//! Ported from Industrial Foregoing: Souls 1.21.1-1.10.7 (`InnovativeOnlineIndustries/
//! Industrial-Foregoing-Souls`, MIT). The mod is one soul economy in three blocks, and this file is
//! its glue: every number comes from [`runtime`], and nothing but the shape of the game call is
//! decided here.
//!
//! | upstream | here |
//! |---|---|
//! | `SoulLaserBaseBlockEntity` (a `ProgressBarComponent` of 20 ticks + one catalyst slot) | [`Laser`]: [`BlockEntityPart::block_entity_tick`] walks the bar, [`BlockPart::use_item_on`] seats the lens, [`BlockPart::use_without_item`] takes it back |
//! | `SoulPipeBlockEntity` + Titanium's `SoulNetwork` | [`Pipe`]: the network is the connected pipes, each holding up to four souls ([`runtime::Network`]) |
//! | `SoulSurgeBlockEntity` | [`Surge`]: buys 300 accelerated ticks with one soul and random-ticks the block it is mounted on |
//! | `collectImplicitComponents` (a machine's contents on its item) | [`Hooks::block_drops`] writes a **fresh** `minecraft:block_entity_data` payload on the drop and [`SoulMachineItem::on_block_placed_by`] merges it back, clamped |
//!
//! # What the pin cannot express, and what this port does instead
//!
//! - **No capability for a mod's own soul handler.** `SoulCapabilities.BLOCK` is NeoForge's custom
//!   capability; Horizon has no cross-mod capability registration, so the pipe recognises the one
//!   handler this mod ships by reading its block entity's saved compound (the four APIs a port has
//!   for that are `Hooks::block_entity`-shaped reads). A future soul handler in another mod would
//!   need a capability API.
//! - **No way to tick another block entity.** The surge's block-entity branch (`ACCELERATION_TICK`
//!   ticks of a machine in front of it) is the mod's headline use and it is not expressible:
//!   `blocked(no-tick-another-block-entity)`. Its plain-block branch is (API: `level_block_random_tick`).
//! - **No way to tick an entity.** `blocked(no-entity-tick)`; the air branch therefore does nothing.
//! - **No comparator read of a block entity's data from `analog_output`** (the hook gets a
//!   `LevelHost`, which cannot resolve a block entity by position):
//!   `blocked(analog-output-needs-block-entity-read)`. The formula is in
//!   [`runtime::LaserBase::analog_output`] with its test, ready for the hook.
//! - **A block state cannot be rewritten from a tick or a neighbour change without `set_block`**,
//!   which recreates the block entity, so the surge's `enabled`/connection properties are not
//!   declared and its redstone rule is read in the tick instead of a state.
//! - **The engine cannot put a mod block entity's data back from its item.**
//!   `block_entity_item::place` merges the item's `minecraft:block_entity_data` and then
//!   re-canonicalises the result; `block_entity_nbt::canonical` answers `None` for a block entity id
//!   it does not model, so the merge is dropped. This package writes the payload in `block_drops`
//!   and restores it in `on_block_placed_by` instead (slice 3).
//!
//! # The harness commands
//!
//! Upstream registers no commands. The laser base has its own menu and screen (slice 0.2.4); the
//! pipe and the surge have none (Titanium's screens are the parent's), and the numbers behind the
//! menu are also read and set through `/industrialforegoingsouls` — the precedent is
//! `/powah info|set` and the parent port's own `/industrialforegoing info`. `info` is for everyone;
//! `souls` and `wait` are operator commands, and `info` prints the menu's data slots.
pub mod runtime;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use mod_api::{
    BlockDropsEvent, BlockEntityPart, BlockEntityRef, BlockEntityTickEvent, BlockEntityTicksEvent,
    BlockEntityTypeKey, BlockPart, BlockState, ClientHost, Command, CommandArgumentValue,
    CommandInvocation, Compound, Direction, EntityRef, Flow, GameEventContext, Hand, Hooks, Host,
    HostTransaction, Identifier, InteractionResult, ItemKey, ItemPart, ItemStack, LevelHost,
    LevelRef, MenuContainer, MenuPart, MenuSlotRulesEvent, ModContainer, OnBlockPlacedByEvent,
    OnDestroyedByPlayerEvent, RandomSource, RegisterCommandsEvent, Registrar, RegistrationContext,
    RunModCommandEvent, Screen, ScreenDrawEvent, ScreenEvent, ServerHost, SoundSource,
    StateForPlacementEvent, Tag, Text, UseItemOnEvent, UseWithoutItemEvent,
};

use crate::runtime::{
    AUGMENT_SLOTS, Addon, LENS_SLOT, LaserBase, MAX_NETWORK_PIPES, MAX_PROGRESS, MENU, Network,
    NetworkCache, SLOT_COUNT, SOUL_STORAGE, Surge, Target, Walked, WaterClick, Waterlogged,
    adopted_items, augment_slot, grow_component, lens_fits, menu_data, pipe_pull, replacement_slot,
    slot_accepts, water_click,
};

/// The mod's three blocks.
const BASE: &str = "industrialforegoingsouls:soul_laser_base";
const PIPE: &str = "industrialforegoingsouls:soul_network_pipe";
const SURGE: &str = "industrialforegoingsouls:soul_surge";
/// The mod command's root word.
const COMMAND: &str = "industrialforegoingsouls";
/// The laser base's saved keys: the tank, the bar and the seated lens.
const KEY_SOULS: &str = "souls";
const KEY_PROGRESS: &str = "progress";
const KEY_LENS: &str = "lens";
/// The bar's own maximum, saved the way `ProgressBarComponent` saves `maxProgress`, so an
/// efficiency augment's rescale survives a reload.
const KEY_BAR: &str = "bar";
/// Titanium's augment component (`AugmentWrapper.ATTACHMENT`): the data component an addon item
/// carries, registered by the parent package.
const AUGMENT_COMPONENT: &str = "titanium:augments";
/// The two values that component carries (`AugmentTypes`). The machine refuses `Speed` outright
/// (`canAcceptAugment`) and reads `Efficiency`.
const SPEED: &str = "Speed";
const EFFICIENCY: &str = "Efficiency";
/// The item the seated augment came from: the carried payload's record, since the payload is a
/// saved compound and the slot's stack is not part of it.
const KEY_AUGMENT_ITEM: &str = "augment_item";
/// The surge's saved key: the accelerated ticks it still owns.
const KEY_WAIT: &str = "wait";
/// The flag a machine sets on its first tick, once a 0.2.x save's `lens`/`augments` keys have been
/// adopted into the container's slots. Absent means "not adopted yet".
const KEY_SLOTS: &str = "slots";
/// The container's stacks in a carried payload: each a stack's own SNBT with its `Slot`. The only
/// record of the slots the payload needs — the saved compound is a compound, and the slots are not
/// part of it.
const KEY_CARRIED: &str = "carried";
/// The surge's six-way state property (Titanium's `RotationType.SIX_WAY`).
const FACING: &str = "facing";
/// The two tags a surge must not accelerate: the mod's own and the convention tag.
const CANT_ACCELERATE: [&str; 2] = [
    "industrialforegoingsouls:cant_accelerate",
    "c:tick_acceleration_disallowed",
];
/// The warden, and the tag the ecosystem's other wardens live in.
const WARDEN: &str = "minecraft:warden";
const WARDENS: &str = "c:wardens";
/// `minecraft:generic` is what `DamageSources.generic` is, and it exists at this pin.
const GENERIC: &str = "minecraft:generic";
const AIR: &str = "minecraft:air";

/// The block entity types the tick, the lens seat and the network read all key on, resolved once in
/// `register` (a Part is called per object and gets no registrar).
static BASE_KIND: OnceLock<BlockEntityTypeKey> = OnceLock::new();
static PIPE_KIND: OnceLock<BlockEntityTypeKey> = OnceLock::new();
static SURGE_KIND: OnceLock<BlockEntityTypeKey> = OnceLock::new();
static WARDEN_KIND: OnceLock<mod_api::EntityTypeKey> = OnceLock::new();
/// The one item a laser base seats, resolved in `register` so a click can compare against it.
static LENS_ITEM: OnceLock<ItemKey> = OnceLock::new();
/// The two buckets a waterlogged machine trades: a full one fills it, an empty one drains it. Both
/// are resolved in `register` so the click only moves keys.
static BUCKET_ITEM: OnceLock<ItemKey> = OnceLock::new();
static WATER_BUCKET_ITEM: OnceLock<ItemKey> = OnceLock::new();
/// The three items that place the machines. `block_drops` is given a read-only level host and no
/// game host, so the item it writes the carried payload onto is resolved once in `register`.
static BASE_ITEM: OnceLock<ItemKey> = OnceLock::new();
static PIPE_ITEM: OnceLock<ItemKey> = OnceLock::new();
static SURGE_ITEM: OnceLock<ItemKey> = OnceLock::new();
/// The network walks of the current tick: each connected component is walked once and reused by
/// the pipes that tick in it (see [`with_pipe_network`]). A placement or a break drops it.
static WALKS: Mutex<Option<NetworkCache>> = Mutex::new(None);

/// The mod has no settings: everything it keeps is in the block entities' saved compounds.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the three blocks' block entity types and the Parts that tick them.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let base_block = r.block_key(BASE);
        let _ = r.block_key(PIPE);
        let _ = r.block_key(SURGE);
        let base_item = r.item_key(BASE);
        let pipe_item = r.item_key(PIPE);
        let surge_item = r.item_key(SURGE);
        let _ = LENS_ITEM.set(r.item_key(runtime::LENS));
        let _ = BUCKET_ITEM.set(r.item_key(runtime::BUCKET));
        let _ = WATER_BUCKET_ITEM.set(r.item_key(runtime::WATER_BUCKET));
        let _ = WARDEN_KIND.set(r.entity_type_key(WARDEN));
        // `block_drops` writes the carried payload onto these three items and is given no game
        // host, so they are resolved here; the placement hook that puts the payload back is
        // attached to the same items.
        let _ = (
            BASE_ITEM.set(base_item),
            PIPE_ITEM.set(pipe_item),
            SURGE_ITEM.set(surge_item),
        );
        for item in [base_item, pipe_item, surge_item] {
            r.item_part(item, SoulMachineItem);
        }
        // The verbs below keep the keys: a Part attached to a block entity type is also attached to
        // that type's blocks (`expand_block_entity_parts`), which is what puts `use_item_on` on the
        // laser base's block. A `block_part` call for the same block as well attaches it twice and
        // the engine refuses the whole package.
        let base_kind = r
            .block_entity_type("soul_laser_base")
            .blocks(&["soul_laser_base"])
            .build();
        let pipe_kind = r
            .block_entity_type("soul_network_pipe")
            .blocks(&["soul_network_pipe"])
            .build();
        let surge_kind = r
            .block_entity_type("soul_surge")
            .blocks(&["soul_surge"])
            .build();
        r.block_entity_part(&base_kind, Laser)
            .expect("the laser base ticks its own block entity type");
        r.block_entity_part(&pipe_kind, Pipe)
            .expect("the pipe ticks its own block entity type");
        r.block_entity_part(&surge_kind, SurgePart)
            .expect("the surge ticks its own block entity type");
        // The machine's container and its menu (API 16 + 85): three slots — the catalyst and
        // Titanium's two augment slots — which the menu type the content declares shows as slot
        // 300, 301 and 302. The faces reach nothing: upstream's catalyst is `FaceMode.NONE` on all
        // six sides and Titanium's augment inventory is the GUI's, so no hopper may touch any of it.
        r.block_container(&base_block, container());
        r.menu_part(MENU, SoulMenu);
        // The keys are kept for the command path, which must prove a position holds one of this
        // mod's block entities without swapping a level.
        let _ = (
            BASE_KIND.set(base_kind),
            PIPE_KIND.set(pipe_kind),
            SURGE_KIND.set(surge_kind),
        );
    }

    /// A broken machine's own item carries what the machine held — the tank and bar, the seated
    /// lens and augment, a pipe's souls, a surge's bought ticks — so placing it puts them back.
    ///
    /// Upstream's machines keep their contents on the item the same way
    /// (`collectImplicitComponents`); the pin's engine cannot do it for a **mod** block entity,
    /// because `block_entity_item::place` re-canonicalises the merged data and `canonical` answers
    /// `None` for an id it does not model, so the payload is written here and restored by
    /// [`SoulMachineItem`]. An empty machine's item stays plain.
    ///
    /// The laser base's three slots are a real container now, and the engine spills a container on
    /// break (`PreRemove`): [`Laser::on_destroyed_by_player`] has already moved them aside into
    /// [`take_stash`] and emptied the slots, so the ground gets nothing and the payload carries
    /// them. A break that does not harvest (a creative one) drops nothing, so nothing is stashed and
    /// the engine's own spill is what a creative player sees.
    fn block_drops(&self, level: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) -> Flow {
        let Some(block) = level.state_block(event.state) else {
            return Flow::Continue;
        };
        let Some(machine) = Machine::from_block(block) else {
            return Flow::Continue;
        };
        // A pipe leaving the world changes the component the next tick walks.
        forget_networks();
        // An explosion has already removed the block entity; only a break that still holds it (a
        // player's, a mob's) can carry the contents.
        let Some(entity) = event.block_entity else {
            return Flow::Continue;
        };
        let Some(saved) = level.block_entity_data(entity) else {
            return Flow::Continue;
        };
        let carried = if machine == Machine::Base {
            take_stash(
                event.dimension.key().id(),
                [event.pos.x, event.pos.y, event.pos.z],
            )
        } else {
            None
        };
        let Some(patch) = carried_patch(machine, &saved, carried) else {
            return Flow::Continue;
        };
        let Some(item) = machine.item() else {
            return Flow::Continue;
        };
        for drop in event.drops.iter_mut() {
            if drop.stack.item() == item {
                let _ = drop.stack.apply_components(&patch);
            }
        }
        Flow::Continue
    }

    /// The harness commands (upstream has none).
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::new(
            "industrialforegoingsouls info <pos:block_pos>",
        ));
        event.commands.push(Command::op(
            "industrialforegoingsouls souls <pos:block_pos> <amount:integer>",
            2,
        ));
        event.commands.push(Command::op(
            "industrialforegoingsouls wait <pos:block_pos> <ticks:integer>",
            2,
        ));
    }

    /// Those commands' bodies, on the typed invocation: the dispatcher has already parsed and
    /// resolved every argument.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let invocation = event.command;
        if invocation.literals.first().map(String::as_str) != Some(COMMAND) {
            return Flow::Continue;
        }
        let Some(sub) = invocation.literals.get(1).map(String::as_str) else {
            say(host, "usage: /industrialforegoingsouls info|souls|wait ...");
            return Flow::Cancel;
        };
        let Some(pos) = block_pos(invocation) else {
            say(host, &usage(sub));
            return Flow::Cancel;
        };
        match sub {
            "info" => {
                let line = readout(host, pos);
                say(host, &line);
            }
            "souls" => {
                let Some(amount) = integer(invocation, "amount") else {
                    say(host, &usage(sub));
                    return Flow::Cancel;
                };
                let line = set_souls(host, pos, amount.max(0));
                say(host, &line);
            }
            "wait" => {
                let Some(ticks) = integer(invocation, "ticks") else {
                    say(host, &usage(sub));
                    return Flow::Cancel;
                };
                let line = set_wait(host, pos, ticks.max(0));
                say(host, &line);
            }
            other => say(host, &usage(other)),
        }
        Flow::Cancel
    }
}

// ---------------------------------------------------------------------------
// The Soul Laser Base
// ---------------------------------------------------------------------------

/// The laser base's Part: the 20-tick bar, the warden it wounds, and the one catalyst slot.
struct Laser;

#[mod_api::horizon_part]
impl BlockPart for Laser {
    /// Right-clicking the machine with the blue lens puts it in the catalyst slot, taking one out
    /// of the hand; a stack carrying Titanium's `titanium:augments` component goes into the first
    /// free augment slot the same way. The slots are the machine's own container (three slots,
    /// shown by its menu); a click with anything else falls through to [`Self::use_without_item`],
    /// which opens that menu.
    ///
    /// `canAcceptAugment` refuses a **speed** augment on this machine (it has nothing to speed up:
    /// the bar is the harvest's only clock) and refuses a second augment of a type already seated;
    /// both answers leave the stack in the hand, exactly as the GUI's slot would. A second lens is
    /// a swap, as the GUI's cursor would be.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        let click = event.click;
        let stack = click.stack?;
        if stack.count() == 0 {
            return None;
        }
        let dimension = click.dimension.key().id();
        let level = host.level(dimension)?;
        let entity = host.block_entity(level, click.pos)?;
        if entity.kind(&*host) != Some(*BASE_KIND.get()?) {
            return None;
        }
        let hand = match click.hand {
            Hand::Main => u32::try_from(host.player(click.player)?.selected_slot(&*host)?).ok()?,
            Hand::Off => 40,
        };
        if stack.item() == *LENS_ITEM.get()? {
            // A second lens is a swap, not a void: work out where the seated one goes (and that it
            // is a real item) **before** anything is written, so a full inventory refuses the
            // click instead of losing it.
            let seated = entity
                .item_slot(&*host, slot_index(LENS_SLOT))
                .ok()
                .flatten();
            let (previous, destination) = match seated {
                Some(held) => {
                    let item = held.item();
                    let occupied: Vec<u32> = host
                        .player(click.player)?
                        .inventory_slots(&*host)?
                        .iter()
                        .map(|(slot, _)| *slot)
                        .collect();
                    (
                        Some(item),
                        Some(replacement_slot(&occupied, hand, stack.count())?),
                    )
                }
                None => (None, None),
            };
            let seated_lens = ItemStack::new(*LENS_ITEM.get()?, 1)?;
            let left = stack.clone().shrink(1);
            let player = click.player;
            host.transaction(|host, tx| -> Result<(), ()> {
                entity
                    .set_item_slot(host, tx, slot_index(LENS_SLOT), Some(seated_lens))
                    .map_err(|_| ())?;
                let player = host.player(player).ok_or(())?;
                player
                    .set_inventory_slot(host, hand, left.clone())
                    .map_err(|_| ())?;
                if let (Some(item), Some(destination)) = (previous, destination) {
                    let old = ItemStack::new(item, 1).ok_or(())?;
                    player
                        .set_inventory_slot(host, destination, Some(old))
                        .map_err(|_| ())?;
                }
                Ok(())
            })
            .ok()?;
            return Some(InteractionResult::Consume);
        }
        let addon = addon_of(&*host, stack);
        let slots = container_of(&*host, entity);
        let empty = [
            slots[AUGMENT_SLOTS[0]].is_none(),
            slots[AUGMENT_SLOTS[1]].is_none(),
        ];
        let seated = slots
            .iter()
            .flatten()
            .any(|held| addon_of(&*host, held) == Addon::Efficiency);
        let slot = augment_slot(empty, addon, seated)?;
        let placed = ItemStack::new(stack.item(), 1)?;
        let left = stack.clone().shrink(1);
        let player = click.player;
        host.transaction(|host, tx| -> Result<(), ()> {
            entity
                .set_item_slot(host, tx, slot_index(slot), Some(placed))
                .map_err(|_| ())?;
            host.player(player)
                .ok_or(())?
                .set_inventory_slot(host, hand, left.clone())
                .map_err(|_| ())?;
            Ok(())
        })
        .ok()?;
        Some(InteractionResult::Consume)
    }

    /// The block's use without an item use of its own: opens the machine's menu, as a machine's
    /// `getMenuProvider` does. This is also the fall-through of a click whose item the machine
    /// refused (a speed addon), so a player who tries to insert the wrong item sees the slots.
    ///
    /// The engine's own click path does not open a mod container's content menu at this pin
    /// (measured by `bibliocraft`; `rftoolsstorage` and this port's siblings open theirs the same
    /// way), and a block whose template is no container never reaches it.
    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        let click = event.click;
        let dimension = click.dimension.key().id();
        let level = host.level(dimension)?;
        let entity = host.block_entity(level, click.pos)?;
        if entity.kind(&*host) != Some(*BASE_KIND.get()?) {
            return None;
        }
        if !host
            .player(click.player)?
            .open_menu(host, entity)
            .unwrap_or(false)
        {
            return None;
        }
        Some(InteractionResult::Consume)
    }

    /// A player is about to break the machine: move its three slots aside into [`stash_carried`] and
    /// empty them, so the engine's container spill finds nothing and the drop's payload carries the
    /// parts instead (the ground would otherwise get the lens twice — once spilled, once on the
    /// item). A break that harvests nothing (a creative one) leaves the slots to vanilla's spill.
    fn on_destroyed_by_player(
        &self,
        host: &mut dyn ServerHost,
        event: &mut OnDestroyedByPlayerEvent<'_>,
    ) -> Option<bool> {
        if event.will_harvest {
            stash_carried(host, event.pos);
        }
        None
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Laser {
    /// The machine ticks on the server (`EntityBlock.getTicker`).
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: walk the bar, and on its last tick wound the warden this machine covers and
    /// store the souls of the wound. The bar's length is the seated efficiency augment's rescale,
    /// read from the augment slots, and the menu's data slots are the tank and the bar.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        if event.entity.items.len() != SLOT_COUNT {
            event.entity.items.resize(SLOT_COUNT, None);
        }
        let mut saved = std::mem::take(&mut event.entity.nbt);
        // A save from 0.2.x kept the lens and the augment's item id under its own keys and no
        // slots. Adopt them once, so a machine placed from such a save keeps its parts.
        if !read_flag(&saved, KEY_SLOTS) {
            adopt_legacy(&*host, &saved, &mut event.entity.items);
            write_flag(&mut saved, KEY_SLOTS);
        }
        let lens = event.entity.items[LENS_SLOT]
            .as_ref()
            .is_some_and(|stack| lens_fits(&stack.item().id(&*host)));
        let efficiency = AUGMENT_SLOTS
            .iter()
            .filter_map(|slot| event.entity.items[*slot].as_ref())
            .find_map(|stack| efficiency_of(&augment_component(&*host, stack)?));
        let mut base = LaserBase::from_saved(
            read_int(&saved, KEY_SOULS),
            read_progress(&saved),
            read_bar(&saved),
        );
        let warden = warden_near(host, level, event.pos);
        let health = warden.and_then(|entity| living_health(&*host, entity));
        if base.tick(lens, health, efficiency)
            && let Some(entity) = warden
        {
            let _ = entity.hurt(host, GENERIC, runtime::DAMAGE_PER_OPERATION, None, None);
        }
        if read_int(&saved, KEY_SOULS) != i32::try_from(base.souls).unwrap_or(i32::MAX) {
            write_int(
                &mut saved,
                KEY_SOULS,
                i32::try_from(base.souls).unwrap_or(0),
            );
        }
        if read_progress(&saved)
            != Some(i32::try_from(base.progress).unwrap_or(MAX_PROGRESS as i32))
        {
            write_int(
                &mut saved,
                KEY_PROGRESS,
                i32::try_from(base.progress).unwrap_or(0),
            );
        }
        if read_bar(&saved) != Some(i32::try_from(base.bar).unwrap_or(i32::MAX)) {
            write_int(&mut saved, KEY_BAR, i32::try_from(base.bar).unwrap_or(0));
        }
        event.entity.nbt = saved;
        // The menu's data slots (API 85): the values its screen draws.
        let data = menu_data(&base);
        if event.entity.data != data {
            event.entity.data = data.to_vec();
        }
    }

    /// The machine's menu layout (API 19): the two augment slots beside the catalyst, the tank and
    /// the bar printed over the panel, and the player's inventory beneath.
    fn screen(&self, _host: &dyn Host, event: &mut ScreenEvent<'_>) -> Option<Screen> {
        (event.block == BASE).then(screen_layout)
    }

    /// The screen's readouts, from the menu's data slots `[souls, progress, bar]`: the tank and the
    /// bar. The plain panel and the slot frames are Horizon's.
    fn screen_draw(&self, _host: &dyn ClientHost, event: &mut ScreenDrawEvent<'_>) {
        if event.block != BASE {
            return;
        }
        let souls = event.data.first().copied().unwrap_or(0);
        let progress = event.data.get(1).copied().unwrap_or(0);
        let bar = event.data.get(2).copied().unwrap_or(0);
        let [x, y] = event.origin;
        let tank = format!("{souls} / {SOUL_STORAGE} souls");
        let cycle = format!("{progress} / {bar}");
        event
            .draw
            .text(&tank, [x + 8.0, y + 8.0], 0xFF404040, false);
        event
            .draw
            .text(&cycle, [x + 92.0, y + 8.0], 0xFF404040, false);
        event
            .draw
            .text("catalyst", [x + 98.0, y + 56.0], 0xFF404040, false);
        event
            .draw
            .text("augments", [x + 8.0, y + 56.0], 0xFF404040, false);
    }
}

/// The machine's container: three slots (the catalyst at 0, the augment slots at 1 and 2), the menu
/// type the content declares, and **no** automation access — upstream's catalyst is `FaceMode.NONE`
/// on all six faces and Titanium's augment inventory is the GUI's, so a hopper reaches none of them.
fn container() -> ModContainer {
    ModContainer {
        slots: SLOT_COUNT,
        faces: std::array::from_fn(|_| Vec::new()),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some(MENU.to_owned()),
    }
}

/// The machine's menu layout: upstream's two augment slots either side of the catalyst, under a
/// 176 x 166 panel with the player's inventory at vanilla's place.
fn screen_layout() -> Screen {
    Screen {
        width: 176.0,
        height: 166.0,
        slots: vec![Some([80.0, 38.0]), Some([26.0, 38.0]), Some([134.0, 38.0])],
        inventory_x: 8.0,
        inventory_y: 84.0,
        own_labels: false,
        ..Screen::default()
    }
}

/// The machine's menu rules (`native_menu_slot_rules`, API 130): the same `canAcceptAugment` and
/// catalyst gate the click applies, so a menu cannot seat what a click refuses. A click that is
/// refused leaves the stack on the cursor.
struct SoulMenu;

#[mod_api::horizon_part]
impl MenuPart for SoulMenu {
    fn menu_slot_rules(&self, host: &dyn Host, event: &mut MenuSlotRulesEvent<'_>) {
        if event.menu != MENU || event.container != MenuContainer::Main {
            return;
        }
        let slots = match event.context.pos {
            Some(pos) => slots_at(host, pos),
            None => [None, None, None],
        };
        let seated = slots
            .iter()
            .flatten()
            .any(|held| addon_of(host, held) == Addon::Efficiency);
        let lens = event.stack.item().id(host) == runtime::LENS;
        let addon = addon_of(host, event.stack);
        event.may_place = slot_accepts(event.slot, lens, addon, seated);
        event.max_stack = 1;
    }
}

/// The three container slots of the machine at `pos`, read through the host; all empty when there
/// is no machine there.
fn slots_at(host: &dyn Host, pos: [i32; 3]) -> [Option<ItemStack>; SLOT_COUNT] {
    let Some(level) = host.level(host.dimension()) else {
        return [None, None, None];
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return [None, None, None];
    };
    container_of(host, entity)
}

/// The three container slots of `entity`.
fn container_of(host: &dyn Host, entity: BlockEntityRef) -> [Option<ItemStack>; SLOT_COUNT] {
    std::array::from_fn(|slot| entity.item_slot(host, slot_index(slot)).ok().flatten())
}

/// A slot number as the item-slot API takes it.
fn slot_index(slot: usize) -> u32 {
    u32::try_from(slot).unwrap_or(u32::MAX)
}

/// What a stack is, for the augment slots: the `titanium:augments` component's two values.
fn addon_of(host: &dyn Host, stack: &ItemStack) -> Addon {
    let Some(component) = augment_component(host, stack) else {
        return Addon::None;
    };
    if has_speed(&component) {
        Addon::Speed
    } else if efficiency_of(&component).is_some() {
        Addon::Efficiency
    } else {
        Addon::None
    }
}

/// One break's stash: the container slots the break emptied, and where and when.
struct Stash {
    dimension: String,
    pos: [i32; 3],
    entries: Vec<Tag>,
    at: std::time::Instant,
}

/// How long a stash waits for its drop. `block_drops` runs in the same break, moments later; a
/// stash nobody claimed (a removal the level refused) expires rather than leaking.
const STASH_LIFETIME: std::time::Duration = std::time::Duration::from_secs(5);

/// The breaks that have emptied a machine's container and are waiting for the drop.
static STASHES: Mutex<Vec<Stash>> = Mutex::new(Vec::new());

/// Move the machine's three slots aside for the drop that follows, and empty them, so the engine's
/// own container spill (`PreRemove`'s `Contents`) finds nothing: what the machine held travels on
/// its item, not on the ground beside it. The stacks are kept as SNBT with their `Slot`, so an
/// augment's own `titanium:augments` component (a `/give` override included) travels too.
fn stash_carried(host: &mut dyn ServerHost, pos: [i32; 3]) {
    let Some(base) = BASE_KIND.get().copied() else {
        return;
    };
    if host.dimension().is_empty() {
        return;
    }
    let dimension = host.dimension().to_owned();
    let Some(level) = host.level(&dimension) else {
        return;
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return;
    };
    if entity.kind(&*host) != Some(base) {
        return;
    }
    let mut entries = Vec::new();
    let mut filled = Vec::new();
    for slot in 0..SLOT_COUNT {
        let index = slot_index(slot);
        let Some(stack) = entity.item_slot(&*host, index).ok().flatten() else {
            continue;
        };
        let Some(mut entry) = stack.to_nbt(&*host) else {
            continue;
        };
        stamp_effective_augment(&*host, &stack, &mut entry);
        let Ok(written) = i8::try_from(slot) else {
            continue;
        };
        entry.insert("Slot".into(), Tag::Byte(written));
        entries.push(Tag::Compound(entry));
        filled.push(index);
    }
    if entries.is_empty() {
        return;
    }
    let cleared = host
        .transaction(|host, tx| -> Result<(), ()> {
            for slot in filled {
                entity.set_item_slot(host, tx, slot, None).map_err(|_| ())?;
            }
            entity.set_changed(host).map_err(|_| ())?;
            Ok(())
        })
        .is_ok();
    if !cleared {
        return;
    }
    let mut stashes = STASHES.lock().expect("the stash lock is poisoned");
    stashes.retain(|stash| stash.at.elapsed() < STASH_LIFETIME);
    stashes.push(Stash {
        dimension,
        pos,
        entries,
        at: std::time::Instant::now(),
    });
}

/// Write an addon's effective `titanium:augments` into its carried SNBT, as upstream's
/// `collectImplicitComponents` puts a stack's implicit components on the item: the item definition
/// declares the value, so a payload that carried only the id would depend on that definition never
/// changing. A stack that already carries the component keeps its own (an override a `/give` set).
fn stamp_effective_augment(host: &dyn Host, stack: &ItemStack, entry: &mut Compound) {
    let Some(component) = augment_component(host, stack) else {
        return;
    };
    let key = COMPONENTS.into();
    if !matches!(entry.get(&key), Some(Tag::Compound(_))) {
        entry.insert(key.clone(), Tag::Compound(Compound::new()));
    }
    if let Some(Tag::Compound(components)) = entry.get_mut(&key) {
        components
            .entry(AUGMENT_COMPONENT.into())
            .or_insert(Tag::Compound(component));
    }
}

/// The slots the break at `pos` moved aside, if that break was this machine's.
fn take_stash(dimension: &str, pos: [i32; 3]) -> Option<Vec<Tag>> {
    let mut stashes = STASHES.lock().expect("the stash lock is poisoned");
    let index = stashes
        .iter()
        .position(|stash| stash.dimension == dimension && stash.pos == pos)?;
    let stash = stashes.remove(index);
    (stash.at.elapsed() < STASH_LIFETIME).then_some(stash.entries)
}

/// Adopt a 0.2.x save's keys into the container's slots, once: the lens at slot 0 and the augment's
/// item at slot 1. A slot that already holds something is left alone, and an id the registry does
/// not know is dropped rather than turned into a placeholder.
fn adopt_legacy(host: &dyn Host, saved: &Compound, items: &mut [Option<ItemStack>]) {
    let migrated = adopted_items(
        read_lens(saved).as_deref(),
        [read_augment_item(saved).as_deref(), None],
    );
    for (slot, item) in migrated.iter().enumerate() {
        let Some(item) = item else { continue };
        if items.get(slot).is_some_and(Option::is_some) {
            continue;
        }
        let Ok(id) = Identifier::parse(item) else {
            continue;
        };
        let Some(key) = host.item_key(&id) else {
            continue;
        };
        let Some(stack) = ItemStack::new(key, 1) else {
            continue;
        };
        if let Some(entry) = items.get_mut(slot) {
            *entry = Some(stack);
        }
    }
}

// ---------------------------------------------------------------------------
// The Soul Pipe
// ---------------------------------------------------------------------------

/// The pipe's Part: one network transfer a tick.
struct Pipe;

#[mod_api::horizon_part]
impl BlockPart for Pipe {
    /// Right-clicking a dry pipe with a water bucket fills it (and hands back the empty bucket);
    /// an empty bucket on a waterlogged one takes the water out. Upstream's `SoulPipeBlock` is a
    /// `SimpleWaterloggedBlock`, and this click is how its water goes in and out at this pin — see
    /// [`waterlog_click`] and the gap `notes.md` records for the interface itself.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        waterlog_click(host, event)
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Pipe {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// `SoulPipeBlockEntity.serverTick`: for each of the six sides, push up to four into a soul
    /// handler that takes them, and where it took none, pull up to four out of it.
    ///
    /// This mod's only handler (the laser base) refuses to be filled — `SLBSoulCap.fill` returns 0 —
    /// so in a package of this mod alone the pull is the whole transfer. That is not a shortcut: it
    /// is the same branch upstream takes against the same handler.
    ///
    /// The connected component is walked **once a tick** and shared by the pipes that tick in it
    /// ([`with_pipe_network`]); only the pipes whose souls actually moved are written back.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        let Some(pipe) = host.block_key(PIPE) else {
            return;
        };
        let own = read_int(&event.entity.nbt, KEY_SOULS).max(0) as u32;
        let Some((_, changed)) =
            with_pipe_network(host, level, event.pos, own, |host, level, network| {
                for direction in Direction::ALL {
                    let offset = direction.normal();
                    let target = [
                        event.pos[0] + offset[0],
                        event.pos[1] + offset[1],
                        event.pos[2] + offset[2],
                    ];
                    if level.block(host, target) == Some(pipe) {
                        continue;
                    }
                    let Some(offered) = handler_souls(host, level, target) else {
                        continue;
                    };
                    let taken = pipe_pull(offered, network);
                    if taken > 0 && !drain_handler(host, level, target, taken) {
                        // The handler refused the write: put the souls back so nothing is
                        // duplicated by a failed drain.
                        let _ = network.drain(taken);
                    }
                }
            })
        else {
            return;
        };
        for (pos, souls) in changed {
            let amount = i32::try_from(souls).unwrap_or(i32::MAX);
            if pos == event.pos {
                write_int(&mut event.entity.nbt, KEY_SOULS, amount);
                continue;
            }
            let Some(entity) = host.block_entity(level, pos) else {
                continue;
            };
            let mut patch = Compound::new();
            write_int(&mut patch, KEY_SOULS, amount);
            let _ = entity.merge_data(host, patch);
        }
    }
}

// ---------------------------------------------------------------------------
// The Soul Surge
// ---------------------------------------------------------------------------

/// The surge's Part: the six-way facing it takes from its placer.
struct SurgePart;

#[mod_api::horizon_part]
impl BlockPart for SurgePart {
    /// A placed surge points the way it was clicked (`context.getClickedFace`), and it accelerates
    /// the block behind that face.
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let turned =
            level.state_with_property(event.state, FACING, facing_name(event.context.face))?;
        Some(Some(turned))
    }

    /// A surge is a `SimpleWaterloggedBlock` too, so a bucket click fills or drains it exactly as it
    /// does a pipe ([`waterlog_click`]).
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        waterlog_click(host, event)
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for SurgePart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// `SoulSurgeBlockEntity.serverTick`: while the surge is not powered, buy 300 accelerated ticks
    /// with one soul and random-tick the block it covers four times at 3% a tick.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        // Upstream keeps an `enabled` block state that `neighborChanged` sets from
        // `hasNeighborSignal`; with no way to rewrite a state outside `set_block` (which would
        // recreate the block entity) the tick asks the signal directly. Same rule, one step later.
        if level.has_neighbor_signal(&*host, event.pos) {
            return;
        }
        let saved = event.entity.nbt.clone();
        let mut surge = Surge::from_saved(read_int(&saved, KEY_WAIT));
        let facing = level
            .block_property(&*host, event.pos, FACING)
            .unwrap_or_default();
        let target_pos = direction(&facing).map(|direction| {
            let offset = direction.opposite().normal();
            [
                event.pos[0] + offset[0],
                event.pos[1] + offset[1],
                event.pos[2] + offset[2],
            ]
        });
        let target = match target_pos {
            Some(pos) if accelerable(host, level, pos) => Target::Block,
            _ => Target::Nothing,
        };
        let (pipes, mut network) = walk(host, level, event.pos, None);
        let roll = level
            .with_random(host, |random| random.next_double())
            .unwrap_or(1.0);
        let step = surge.tick(&mut network, target, roll);
        if let Some(pos) = target_pos {
            for _ in 0..step.random_ticks {
                if !level.block_random_tick(host, pos) {
                    break;
                }
            }
        }
        if step.souls_spent > 0 {
            write_network(host, level, &pipes, &network, None, &mut event.entity.nbt);
        }
        let wait = i32::try_from(surge.ticking_time).unwrap_or(i32::MAX);
        if read_int(&saved, KEY_WAIT) != wait {
            write_int(&mut event.entity.nbt, KEY_WAIT, wait);
        }
    }
}

// ---------------------------------------------------------------------------
// Waterlogging (`SimpleWaterloggedBlock`)
// ---------------------------------------------------------------------------

/// `SimpleWaterloggedBlock`'s two interactions, as the click this port has instead of a GUI: a full
/// bucket fills a dry pipe or surge and hands the empty one back, an empty bucket drains a
/// waterlogged one and hands a full one back. Vanilla's `BucketItem.emptyContents`/`BucketPickup`
/// do the same against a block that carries the interface, with the bucket's own sound and
/// `minecraft:fluid_place`/`minecraft:fluid_pickup`.
///
/// **Why the click and not the interface.** The engine's bucket path asks the block
/// `instanceof SimpleWaterloggedBlock`, and a content block's interface set is copied from its
/// template (`minecraft:chorus_plant` for the pipe, `minecraft:iron_block` for the surge); neither
/// has it, and `connects_to` rewrites the class only. So the state and the water in it are the
/// engine's (`state_fluid_state` reads the `waterlogged` property generically), while the two
/// writes are this hook's. `notes.md` records what that leaves out (a flowing fluid, a dispenser).
///
/// The state is written **before** the hand, so a state this block no longer has leaves the bucket
/// where it was; a creative player keeps the bucket they used, as `ItemStack.consume` does. The
/// drain's full bucket goes to the first free inventory slot rather than the held one: this pin
/// reverts a water bucket written into the held slot from a click (measured; see
/// [`runtime::bucket_destination`]).
fn waterlog_click(
    host: &mut dyn ServerHost,
    event: &mut UseItemOnEvent<'_>,
) -> Option<InteractionResult> {
    let click = event.click;
    let stack = click.stack?;
    if stack.count() == 0 {
        return None;
    }
    let dimension = click.dimension.key().id();
    let level = host.level(dimension)?;
    let stored = level.block_property(&*host, click.pos, runtime::WATERLOGGED);
    let state = Waterlogged::from_property(stored.as_deref());
    let item = stack.item().id(&*host);
    let taken = water_click(&item, state)?;
    let player = host.player(click.player)?;
    let slot = match click.hand {
        Hand::Main => u32::try_from(player.selected_slot(&*host)?).ok()?,
        Hand::Off => 40,
    };
    let creative = player.has_infinite_materials(&*host).unwrap_or(false);
    match taken {
        // A creative player's water bucket is not used up: the hand keeps it, as `ItemStack.consume`
        // leaves an infinite-materials player's stack alone.
        WaterClick::Fill if creative => {
            if !host.set_block_property(
                click.pos,
                runtime::WATERLOGGED,
                Waterlogged::Holding.property(),
            ) {
                return None;
            }
        }
        WaterClick::Fill => {
            let bucket = ItemStack::new(*BUCKET_ITEM.get()?, 1)?;
            if !host.set_block_property(
                click.pos,
                runtime::WATERLOGGED,
                Waterlogged::Holding.property(),
            ) {
                return None;
            }
            player.set_inventory_slot(host, slot, Some(bucket)).ok()?;
        }
        WaterClick::Drain => {
            // Where the water goes must be settled **before** anything is written: a full inventory
            // refuses the click and the block keeps its water.
            let occupied: Vec<u32> = player
                .inventory_slots(&*host)?
                .iter()
                .map(|(slot, _)| *slot)
                .collect();
            let destination = runtime::bucket_destination(&occupied, slot)?;
            let water = ItemStack::new(*WATER_BUCKET_ITEM.get()?, 1)?;
            if !host.set_block_property(
                click.pos,
                runtime::WATERLOGGED,
                Waterlogged::Dry.property(),
            ) {
                return None;
            }
            player.set_inventory_slot(host, slot, None).ok()?;
            player
                .set_inventory_slot(host, destination, Some(water))
                .ok()?;
        }
    }
    let at = [
        f64::from(click.pos[0]) + 0.5,
        f64::from(click.pos[1]) + 0.5,
        f64::from(click.pos[2]) + 0.5,
    ];
    let (sound, game_event) = match taken {
        WaterClick::Fill => ("minecraft:item.bucket.empty", "minecraft:fluid_place"),
        WaterClick::Drain => ("minecraft:item.bucket.fill", "minecraft:fluid_pickup"),
    };
    let _ = level.play_sound(host, sound, at, SoundSource::Blocks, 1.0, 1.0);
    let context = GameEventContext {
        source_entity: Some(player.entity()),
        affected_state: None,
    };
    let _ = level.game_event(host, game_event, at, context);
    Some(InteractionResult::Consume)
}

// ---------------------------------------------------------------------------
// The network walk
// ---------------------------------------------------------------------------

/// The pipes connected to `origin`, and what each holds.
///
/// Upstream's network is Titanium's `block_network` graph of elements; here it is the connected
/// component of pipe blocks, walked over the six sides once per tick (see
/// [`with_pipe_network`]). `own` supplies the souls of the pipe whose own tick is asking — a block
/// entity out for its tick cannot be read — and `None` when the walk starts at a surge. The walk is
/// set-based and stops at [`MAX_NETWORK_PIPES`], so a long line cannot make one tick unbounded.
fn walk(
    host: &mut dyn ServerHost,
    level: LevelRef,
    origin: [i32; 3],
    own: Option<([i32; 3], u32)>,
) -> (Vec<[i32; 3]>, Network) {
    let Some(pipe) = host.block_key(PIPE) else {
        return (Vec::new(), Network::new([]));
    };
    let (seeds, seated) = match own {
        Some((pos, amount)) => (vec![pos], Some((pos, amount))),
        None => {
            let mut seeds = Vec::with_capacity(6);
            for direction in Direction::ALL {
                let offset = direction.normal();
                let next = [
                    origin[0] + offset[0],
                    origin[1] + offset[1],
                    origin[2] + offset[2],
                ];
                if level.block(host, next) == Some(pipe) {
                    seeds.push(next);
                }
            }
            (seeds, None)
        }
    };
    if seeds.is_empty() {
        return (Vec::new(), Network::new([]));
    }
    let mut is_pipe = |pos: [i32; 3]| level.block(host, pos) == Some(pipe);
    let positions = grow_component(seeds, MAX_NETWORK_PIPES, &mut is_pipe);
    let souls: Vec<u32> = positions
        .iter()
        .map(|pos| pipe_souls(host, level, *pos))
        .collect();
    let mut network = Network::new(souls);
    if let Some((pos, amount)) = seated
        && let Some(index) = positions.iter().position(|position| *position == pos)
    {
        network.set(index, amount);
    }
    (positions, network)
}

/// One pipe a tick wrote back: its position and its new soul count.
type Moved = Vec<([i32; 3], u32)>;

/// Run `f` on the network the pipe at `pos` belongs to, walking that component **once this tick**
/// and caching it for the other pipes that tick in it. `own` is the ticking pipe's own souls (its
/// entity is out for its tick and cannot be read).
///
/// Returns what `f` returned and the pipes whose souls moved, with their new values; the caller
/// writes only those back. A placement or a break drops the cache ([`forget_networks`]), so a
/// component never outlives the shape it described.
fn with_pipe_network<R>(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    own: u32,
    f: impl FnOnce(&mut dyn ServerHost, LevelRef, &mut Network) -> R,
) -> Option<(R, Moved)> {
    let pipe = host.block_key(PIPE)?;
    let key = cache_key(level, &*host);
    let mut guard = WALKS.lock().expect("the network cache lock is poisoned");
    let cache = guard.get_or_insert_with(NetworkCache::new);
    cache.begin(key);
    if cache.component(pos).is_none() {
        let mut is_pipe = |target: [i32; 3]| level.block(&*host, target) == Some(pipe);
        let positions = grow_component([pos], MAX_NETWORK_PIPES, &mut is_pipe);
        let souls: Vec<u32> = positions
            .iter()
            .map(|target| pipe_souls(host, level, *target))
            .collect();
        cache.insert(Walked::new(positions, souls));
    }
    let before = cache.component(pos)?.network().pipes().to_vec();
    let mut changed: Moved = Vec::new();
    let result = {
        let walked = cache.component_mut(pos)?;
        walked.set_souls(pos, own);
        let result = f(host, level, walked.network_mut());
        for (index, (position, souls)) in walked
            .positions()
            .iter()
            .zip(walked.network().pipes())
            .enumerate()
        {
            if before.get(index) != Some(souls) {
                changed.push((*position, *souls));
            }
        }
        result
    };
    for (position, souls) in &changed {
        cache.set_souls(*position, *souls);
    }
    Some((result, changed))
}

/// The cache's identity: one open of one level, at one game tick.
fn cache_key(level: LevelRef, host: &dyn Host) -> u64 {
    let mut hasher = DefaultHasher::new();
    level.hash(&mut hasher);
    level.game_time(host).unwrap_or(0).hash(&mut hasher);
    hasher.finish()
}

/// Drop every cached walk: a placed or broken block changes the components.
fn forget_networks() {
    *WALKS.lock().expect("the network cache lock is poisoned") = None;
}

/// Keep the cached component's souls in step with a written network (the surge's spend and the
/// `souls` command use this path).
fn cache_set(pos: [i32; 3], souls: u32) {
    let mut guard = WALKS.lock().expect("the network cache lock is poisoned");
    if let Some(cache) = guard.as_mut() {
        cache.set_souls(pos, souls);
    }
}

/// What the pipe at `pos` holds. A pipe whose block entity is missing (a half-placed chunk) holds
/// nothing but still joins the network, which is what keeps the walk going through it.
fn pipe_souls(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> u32 {
    host.block_entity(level, pos)
        .and_then(|entity| entity.data(&*host))
        .map(|data| read_int(&data, KEY_SOULS).max(0) as u32)
        .unwrap_or(0)
}

/// Write the network back: the pipe whose tick this is writes its own compound, the others are
/// merged into place. A block entity out for its tick cannot be read, but a write to a *different*
/// block entity lands normally — and only that other entity's own write-back could undo it.
fn write_network(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pipes: &[[i32; 3]],
    network: &Network,
    own: Option<[i32; 3]>,
    own_nbt: &mut Compound,
) {
    for (pos, souls) in pipes.iter().zip(network.pipes()) {
        let amount = i32::try_from(*souls).unwrap_or(i32::MAX);
        if Some(*pos) == own {
            write_int(own_nbt, KEY_SOULS, amount);
            cache_set(*pos, u32::try_from(amount).unwrap_or(0));
            continue;
        }
        let Some(entity) = host.block_entity(level, *pos) else {
            continue;
        };
        let mut patch = Compound::new();
        write_int(&mut patch, KEY_SOULS, amount);
        if entity.merge_data(host, patch).unwrap_or(false) {
            cache_set(*pos, u32::try_from(amount).unwrap_or(0));
        }
    }
}

/// The souls the soul handler at `target` offers, when there is one: this mod's only handler is the
/// laser base, recognised by its block entity type.
fn handler_souls(host: &mut dyn ServerHost, level: LevelRef, target: [i32; 3]) -> Option<u32> {
    let base = *BASE_KIND.get()?;
    let entity = host.block_entity(level, target)?;
    if entity.kind(&*host) != Some(base) {
        return None;
    }
    let data = entity.data(&*host)?;
    Some(read_int(&data, KEY_SOULS).max(0) as u32)
}

/// Take `amount` souls out of the handler at `target`; whether the write landed.
fn drain_handler(
    host: &mut dyn ServerHost,
    level: LevelRef,
    target: [i32; 3],
    amount: u32,
) -> bool {
    let Some(entity) = host.block_entity(level, target) else {
        return false;
    };
    let Some(data) = entity.data(&*host) else {
        return false;
    };
    let left = read_int(&data, KEY_SOULS)
        .max(0)
        .saturating_sub(i32::try_from(amount).unwrap_or(i32::MAX));
    let mut patch = Compound::new();
    write_int(&mut patch, KEY_SOULS, left);
    entity.merge_data(host, patch).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// The warden
// ---------------------------------------------------------------------------

/// The first warden inside `Shapes.box(-1, 0, -1, 2, 3, 2)` moved over the machine, hurt or not.
/// Upstream asks for a `LivingEntity` and tests the type; the box is the same one.
fn warden_near(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<EntityRef> {
    let warden = *WARDEN_KIND.get()?;
    let min = [
        f64::from(pos[0]) - 1.0,
        f64::from(pos[1]) - 1.0,
        f64::from(pos[2]) - 1.0,
    ];
    let max = [
        f64::from(pos[0]) + 2.0,
        f64::from(pos[1]) + 2.0,
        f64::from(pos[2]) + 2.0,
    ];
    let entities = level.entities_in(&*host, min, max)?;
    entities.into_iter().find(|entity| {
        let Some(kind) = entity.kind(&*host) else {
            return false;
        };
        kind == warden || host.entity_type_in_tag(&kind.id(&*host), WARDENS)
    })
}

// ---------------------------------------------------------------------------
// The augment slots
// ---------------------------------------------------------------------------

/// The `titanium:augments` component a clicked stack carries: its own override when it has one, else
/// the item's declared default — `AugmentWrapper`'s `stack.getOrDefault(ATTACHMENT, …)` reads the
/// effective component, which is why a `/give` with a hand-written value wins over the item file.
fn augment_component(host: &dyn Host, stack: &ItemStack) -> Option<Compound> {
    if let Some(components) = stack.components()
        && let Some(Tag::Compound(component)) = components.as_nbt().get(&AUGMENT_COMPONENT.into())
    {
        return Some(component.clone());
    }
    match host.item_default_component(stack.item(), AUGMENT_COMPONENT) {
        Some(Tag::Compound(component)) => Some(component),
        _ => None,
    }
}

/// One float an augment component stores: a data pack's JSON number arrives as a `Double`, a
/// `/give`'s SNBT as a `Float` (its bits), a hand-written one as an `Int`.
fn augment_number(component: &Compound, key: &str) -> Option<f32> {
    match component.get(&key.into())? {
        Tag::Float(bits) => Some(f32::from_bits(*bits)),
        Tag::Double(bits) => Some(f64::from_bits(*bits) as f32),
        Tag::Int(value) => Some(*value as f32),
        _ => None,
    }
}

/// The efficiency a component carries, if any: the value the bar is rescaled by.
fn efficiency_of(component: &Compound) -> Option<f32> {
    augment_number(component, EFFICIENCY)
}

/// Whether a component carries speed, which this machine refuses (`canAcceptAugment`: the bar is
/// the harvest's only clock, so there is nothing for speed to shorten).
fn has_speed(component: &Compound) -> bool {
    augment_number(component, SPEED).is_some()
}

// ---------------------------------------------------------------------------
// The machine on its own item (slice 3)
// ---------------------------------------------------------------------------

/// The three block items' Part: placing a machine that came from a broken one puts its contents
/// back.
///
/// The engine cannot do this itself. `block_entity_item::place` merges the item's
/// `minecraft:block_entity_data` into the new block entity and then re-canonicalises the result;
/// `block_entity_nbt::canonical` answers `None` for a block entity id it does not model — every
/// mod's — so the merge is dropped before it reaches the block entity, and
/// `game::block_items::saved_block_entity` never even hands the engine a mod block entity. The
/// payload therefore comes back through this hook, which runs after the block entity exists and is
/// given the placing stack.
struct SoulMachineItem;

#[mod_api::horizon_part]
impl ItemPart for SoulMachineItem {
    /// Merge the carried keys back, clamped to the machine's capacities.
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        // A placed pipe joins a component; the cached walks describe the world before it.
        forget_networks();
        let Some(payload) = item_payload(event.stack) else {
            return;
        };
        let Some(level) = event
            .placer
            .and_then(|player| player.level(&*host))
            .or_else(|| host.level(host.dimension()))
        else {
            return;
        };
        let Some(entity) = host.block_entity(level, event.pos) else {
            return;
        };
        let Some(kind) = entity.kind(&*host) else {
            return;
        };
        let Some(machine) = Machine::of_kind(kind) else {
            return;
        };
        let Some(patch) = restore_patch(machine, &payload) else {
            return;
        };
        // The numbers and the container's stacks go back together: a placement that fails to seat
        // the lens must not leave a tank that disagrees with its slots.
        let carried = if machine == Machine::Base {
            carried_items(&*host, &payload)
        } else {
            Vec::new()
        };
        let mut consumed = Compound::new();
        consumed.insert(KEY_CARRIED.into(), Tag::List(Vec::new()));
        let _ = host.transaction(|host, tx| -> Result<(), ()> {
            entity.merge_data(host, patch.clone()).map_err(|_| ())?;
            for (slot, stack) in carried {
                entity
                    .set_item_slot(host, tx, slot, Some(stack))
                    .map_err(|_| ())?;
            }
            entity.merge_data(host, consumed.clone()).map_err(|_| ())?;
            entity.set_changed(host).map_err(|_| ())?;
            Ok(())
        });
    }
}

/// The `minecraft:block_entity_data` compound a stack carries, if any: the payload
/// `block_drops` wrote on it (vanilla's `TypedEntityData`, `id` and the data beside it).
fn item_payload(stack: &ItemStack) -> Option<Compound> {
    match stack
        .components()?
        .as_nbt()
        .get(&BLOCK_ENTITY_DATA.into())?
    {
        Tag::Compound(payload) if !payload.is_empty() => Some(payload.clone()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The `/industrialforegoingsouls` commands
// ---------------------------------------------------------------------------

/// A usage line for `sub`, as a player who mistyped it is told.
fn usage(sub: &str) -> String {
    match sub {
        "info" => "usage: /industrialforegoingsouls info <x> <y> <z>".to_owned(),
        "souls" => "usage: /industrialforegoingsouls souls <x> <y> <z> <amount>".to_owned(),
        other => format!("usage: /industrialforegoingsouls {other} <x> <y> <z> <ticks>"),
    }
}

/// One feedback line to the command's source.
fn say(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

/// The `pos` argument every usage starts with.
fn block_pos(invocation: &CommandInvocation) -> Option<[i32; 3]> {
    match invocation.argument("pos")? {
        CommandArgumentValue::BlockPos(pos) => Some(*pos),
        _ => None,
    }
}

/// The integer argument `name`.
fn integer(invocation: &CommandInvocation, name: &str) -> Option<i32> {
    match invocation.argument(name)? {
        CommandArgumentValue::Integer(value) => Some(*value),
        _ => None,
    }
}

/// What the block at `pos` is, as the command path sees it: its own block entity kind.
fn our_kind(host: &mut dyn ServerHost, pos: [i32; 3]) -> Option<BlockEntityTypeKey> {
    let level = host.level(host.dimension())?;
    let entity = host.block_entity(level, pos)?;
    entity.kind(&*host)
}

/// The line `/industrialforegoingsouls info` prints: everything the e2e asserts, in one line.
fn readout(host: &mut dyn ServerHost, pos: [i32; 3]) -> String {
    let Some(level) = host.level(host.dimension()) else {
        return format!("{pos:?}: no such level.");
    };
    let time = level.game_time(host).unwrap_or(0);
    let entity = host.block_entity(level, pos);
    let data = entity.and_then(|entity| entity.data(host));
    let kind = entity.and_then(|entity| entity.kind(&*host));
    let (Some(data), Some(_)) = (data, kind) else {
        return format!("{pos:?} is not an industrialforegoingsouls block., t={time}");
    };
    match kind {
        Some(kind) if Some(kind) == BASE_KIND.get().copied() => {
            let base = LaserBase::from_saved(
                read_int(&data, KEY_SOULS),
                read_progress(&data),
                read_bar(&data),
            );
            // The lens and the augment are the container's slots now, not saved keys: the readout
            // reports what the menu shows.
            let entity = entity.expect("the base's block entity was just read");
            let slots = container_of(&*host, entity);
            let lens = slots[LENS_SLOT]
                .as_ref()
                .map(|stack| stack.item().id(&*host))
                .unwrap_or_else(|| "none".to_owned());
            let efficiency = slots
                .iter()
                .flatten()
                .find_map(|stack| efficiency_of(&augment_component(&*host, stack)?))
                .unwrap_or(1.0);
            // The menu's data slots (`entity.data`), the values its screen draws: `[]` until the
            // machine's first tick.
            let menu = entity.data_slots(&*host).unwrap_or_default();
            let warden =
                warden_near(host, level, pos).and_then(|entity| living_health(&*host, entity));
            let warden = match warden {
                Some(health) => format!("{health:.1}"),
                None => "none".to_owned(),
            };
            format!(
                "Soul Laser Base at {pos:?}: souls {}/{SOUL_STORAGE}, progress {}/{}, lens {lens}, augments efficiency {efficiency:.2}, menu {menu:?}, warden {warden}, t={time}",
                base.souls, base.progress, base.bar
            )
        }
        Some(kind) if Some(kind) == PIPE_KIND.get().copied() => {
            let (_, network) = walk(
                host,
                level,
                pos,
                Some((pos, read_int(&data, KEY_SOULS).max(0) as u32)),
            );
            let water = waterlogged_at(host, level, pos);
            format!(
                "Soul Pipe at {pos:?}: network {}/{}, waterlogged {}, t={time}",
                network.total(),
                network.capacity(),
                water.property()
            )
        }
        Some(kind) if Some(kind) == SURGE_KIND.get().copied() => {
            let (_, network) = walk(host, level, pos, None);
            let surge = Surge::from_saved(read_int(&data, KEY_WAIT));
            let facing = level.block_property(host, pos, FACING).unwrap_or_default();
            let powered = level.has_neighbor_signal(&*host, pos);
            let water = waterlogged_at(host, level, pos);
            format!(
                "Soul Surge at {pos:?}: network {}/{}, wait {}/{}, facing {facing}, waterlogged {}, powered {powered}, t={time}",
                network.total(),
                network.capacity(),
                surge.ticking_time,
                runtime::SOUL_TIME,
                water.property()
            )
        }
        _ => format!("{pos:?} is not an industrialforegoingsouls block., t={time}"),
    }
}

/// `/industrialforegoingsouls souls <pos> <amount>`: set the tank of a laser base, or fill the
/// network of a pipe or a surge with `amount` souls (what a test or an operator seeds the economy
/// with, since the machine's own source is a warden).
fn set_souls(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> String {
    let Some(kind) = our_kind(host, pos) else {
        return format!("{pos:?} is not an industrialforegoingsouls block.");
    };
    let Some(level) = host.level(host.dimension()) else {
        return format!("{pos:?}: no such level.");
    };
    if Some(kind) == BASE_KIND.get().copied() {
        let Some(entity) = host.block_entity(level, pos) else {
            return format!("{pos:?}: no block entity.");
        };
        let mut patch = Compound::new();
        write_int(&mut patch, KEY_SOULS, amount.min(SOUL_STORAGE as i32));
        return match entity.merge_data(host, patch) {
            Ok(true) => format!(
                "Soul Laser Base at {pos:?}: souls {}/{SOUL_STORAGE}.",
                amount.min(SOUL_STORAGE as i32)
            ),
            _ => format!("{pos:?}: the write did not land."),
        };
    }
    let own = if Some(kind) == PIPE_KIND.get().copied() {
        host.block_entity(level, pos)
            .and_then(|entity| entity.data(&*host))
            .map(|data| (pos, read_int(&data, KEY_SOULS).max(0) as u32))
    } else {
        None
    };
    let (pipes, mut network) = walk(host, level, pos, own);
    let accepted = network.add(u32::try_from(amount.max(0)).unwrap_or(u32::MAX));
    if accepted == 0 {
        return format!(
            "{pos:?}: the network holds {}/{} souls.",
            network.total(),
            network.capacity()
        );
    }
    let mut scratch = Compound::new();
    write_network(host, level, &pipes, &network, None, &mut scratch);
    format!(
        "network at {pos:?}: {}/{} souls.",
        network.total(),
        network.capacity()
    )
}

/// `/industrialforegoingsouls wait <pos> <ticks>`: set a surge's accelerated ticks.
fn set_wait(host: &mut dyn ServerHost, pos: [i32; 3], ticks: i32) -> String {
    let Some(kind) = our_kind(host, pos) else {
        return format!("{pos:?} is not an industrialforegoingsouls block.");
    };
    if Some(kind) != SURGE_KIND.get().copied() {
        return format!("{pos:?} is not a soul surge.");
    }
    let Some(level) = host.level(host.dimension()) else {
        return format!("{pos:?}: no such level.");
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return format!("{pos:?}: no block entity.");
    };
    let mut patch = Compound::new();
    write_int(&mut patch, KEY_WAIT, ticks);
    match entity.merge_data(host, patch) {
        Ok(true) => format!(
            "Soul Surge at {pos:?}: wait {}/{}.",
            ticks,
            runtime::SOUL_TIME
        ),
        _ => format!("{pos:?}: the write did not land."),
    }
}

// ---------------------------------------------------------------------------
// Small shared pieces
// ---------------------------------------------------------------------------

/// Whether the surge may accelerate the block at `pos`: not air, and not in either
/// `cant_accelerate` tag (upstream checks the same two, its own and the convention one).
fn accelerable(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    let Some(block) = level.block(host, pos) else {
        return false;
    };
    if Some(block) == host.block_key(AIR) {
        return false;
    }
    !CANT_ACCELERATE
        .iter()
        .any(|tag| host.block_in_tag(block, tag) == Some(true))
}

/// A living entity's health, through the handle the host gives for it.
fn living_health(host: &dyn Host, entity: EntityRef) -> Option<f32> {
    let living = host.living(entity)?;
    living.health(host)
}

/// The water a block of this mod holds, read from its own state.
fn waterlogged_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Waterlogged {
    let stored = level.block_property(host, pos, runtime::WATERLOGGED);
    Waterlogged::from_property(stored.as_deref())
}

/// The `facing` value a placement takes from the face it was clicked on.
fn facing_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

/// The direction a saved `facing` value names.
fn direction(name: &str) -> Option<Direction> {
    Direction::ALL
        .into_iter()
        .find(|direction| facing_name(*direction) == name)
}

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

/// A machine saved before it ever ticked has no bar: full, as `ProgressBarComponent` starts it.
fn read_progress(nbt: &Compound) -> Option<i32> {
    match nbt.get(&KEY_PROGRESS.into()) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

/// The bar's saved maximum, if it has one.
fn read_bar(nbt: &Compound) -> Option<i32> {
    match nbt.get(&KEY_BAR.into()) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

/// The item id in the catalyst slot, if one is seated.
fn read_lens(nbt: &Compound) -> Option<String> {
    match nbt.get(&KEY_LENS.into())? {
        Tag::String(item) if !item.to_string_lossy().is_empty() => Some(item.to_string_lossy()),
        _ => None,
    }
}

/// The item id the seated augment came from, for the carried payload's record.
fn read_augment_item(nbt: &Compound) -> Option<String> {
    match nbt.get(&KEY_AUGMENT_ITEM.into())? {
        Tag::String(item) if !item.to_string_lossy().is_empty() => Some(item.to_string_lossy()),
        _ => None,
    }
}

/// Whether a saved flag (an int, written by this package) is set.
fn read_flag(nbt: &Compound, key: &str) -> bool {
    read_int(nbt, key) != 0
}

/// Set a saved flag.
fn write_flag(nbt: &mut Compound, key: &str) {
    write_int(nbt, key, 1);
}

// ---------------------------------------------------------------------------
// The machine on its own item (slice 3)
// ---------------------------------------------------------------------------

/// The item-keyed component map of a stack's SNBT (`components`).
const COMPONENTS: &str = "components";

/// The vanilla component that moves a block entity's saved data on its item
/// (`BlockItem.updateCustomBlockEntityTag`): what a broken machine's drop carries.
const BLOCK_ENTITY_DATA: &str = "minecraft:block_entity_data";

/// One of the mod's three machines, as `block_drops` and the placed item's hook see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Machine {
    Base,
    Pipe,
    Surge,
}

impl Machine {
    const ALL: [Self; 3] = [Self::Base, Self::Pipe, Self::Surge];

    /// The machine a block id names, if it is one of this mod's.
    fn from_block(block: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|machine| machine.block() == block)
    }

    /// Its block id, which is also its block entity type's id (`r.block_entity_type("soul_laser_base")`
    /// registers `industrialforegoingsouls:soul_laser_base`).
    fn block(self) -> &'static str {
        match self {
            Self::Base => BASE,
            Self::Pipe => PIPE,
            Self::Surge => SURGE,
        }
    }

    /// The item that places it, resolved in `register` (`block_drops` is given no game host).
    fn item(self) -> Option<ItemKey> {
        match self {
            Self::Base => BASE_ITEM.get().copied(),
            Self::Pipe => PIPE_ITEM.get().copied(),
            Self::Surge => SURGE_ITEM.get().copied(),
        }
    }

    /// The machine a placed block entity is: the restore touches only its own type.
    fn of_kind(kind: BlockEntityTypeKey) -> Option<Self> {
        if Some(kind) == BASE_KIND.get().copied() {
            Some(Self::Base)
        } else if Some(kind) == PIPE_KIND.get().copied() {
            Some(Self::Pipe)
        } else if Some(kind) == SURGE_KIND.get().copied() {
            Some(Self::Surge)
        } else {
            None
        }
    }
}

/// The `minecraft:block_entity_data` patch a broken machine's item carries.
///
/// The payload is **rebuilt**, never copied: `block_drops` reads the block entity's saved compound
/// as it stands before removal, which still holds the old `x`/`y`/`z`, `keepPacked`, `components`
/// and — for a mod block entity — a possibly stale `Items` list. Only the keys this package owns
/// travel, clamped to their capacities, tagged with the type id vanilla's `TypedEntityData` loads a
/// payload into. `None` for a machine holding nothing, so its item stays plain and placing it is a
/// fresh machine.
fn carried_patch(
    machine: Machine,
    saved: &Compound,
    carried: Option<Vec<Tag>>,
) -> Option<Compound> {
    let mut payload = Compound::new();
    match machine {
        Machine::Base => {
            let base = LaserBase::from_saved(
                read_int(saved, KEY_SOULS),
                read_progress(saved),
                read_bar(saved),
            );
            let carried = carried.unwrap_or_default();
            if base.souls == 0 && carried.is_empty() {
                return None;
            }
            write_int(
                &mut payload,
                KEY_SOULS,
                i32::try_from(base.souls).unwrap_or(i32::MAX),
            );
            write_int(
                &mut payload,
                KEY_PROGRESS,
                i32::try_from(base.progress).unwrap_or(i32::MAX),
            );
            write_int(
                &mut payload,
                KEY_BAR,
                i32::try_from(base.bar).unwrap_or(i32::MAX),
            );
            if !carried.is_empty() {
                payload.insert(KEY_CARRIED.into(), Tag::List(carried));
            }
        }
        Machine::Pipe => {
            let souls = read_int(saved, KEY_SOULS).max(0);
            if souls == 0 {
                return None;
            }
            write_int(
                &mut payload,
                KEY_SOULS,
                souls.min(i32::try_from(runtime::SOUL_PER_PIPE).unwrap_or(i32::MAX)),
            );
        }
        Machine::Surge => {
            let wait = read_int(saved, KEY_WAIT).max(0);
            if wait == 0 {
                return None;
            }
            write_int(
                &mut payload,
                KEY_WAIT,
                wait.min(i32::try_from(runtime::SOUL_TIME).unwrap_or(i32::MAX)),
            );
        }
    }
    payload.insert("id".into(), Tag::String(machine.block().into()));
    let mut patch = Compound::new();
    patch.insert(BLOCK_ENTITY_DATA.into(), Tag::Compound(payload));
    Some(patch)
}

/// The container slots a carried payload holds, as `(slot, stack)`: each entry is a stack's own
/// SNBT with its `Slot`, clamped to one item per slot (the machine's slots hold one) and to the
/// three slots it has. An entry naming another item or a component the stack refuses is dropped.
fn carried_items(host: &dyn Host, payload: &Compound) -> Vec<(u32, ItemStack)> {
    let Some(Tag::List(entries)) = payload.get(&KEY_CARRIED.into()) else {
        return Vec::new();
    };
    let mut placed = Vec::new();
    for entry in entries {
        let Tag::Compound(entry) = entry else {
            continue;
        };
        let slot = match entry.get(&"Slot".into()) {
            Some(Tag::Byte(slot)) => match u32::try_from(i16::from(*slot)) {
                Ok(slot) if (slot as usize) < SLOT_COUNT => slot,
                _ => continue,
            },
            _ => continue,
        };
        let Some(mut stack) = ItemStack::from_nbt(host, entry) else {
            continue;
        };
        if stack.set_count(1).is_err() {
            continue;
        }
        placed.push((slot, stack));
    }
    placed
}

/// The keys a carried payload puts back into the machine at the position, clamped to the
/// capacities — a payload hand-written by `/data merge` or carried by another version may hold
/// anything, exactly as upstream's components clamp when they load. `None` when the payload does
/// not name this machine, so a base's payload never reaches a pipe's block entity.
fn restore_patch(machine: Machine, payload: &Compound) -> Option<Compound> {
    if payload_id(payload).as_deref() != Some(machine.block()) {
        return None;
    }
    let mut patch = Compound::new();
    match machine {
        Machine::Base => {
            let base = LaserBase::from_saved(
                read_int(payload, KEY_SOULS),
                read_progress(payload),
                read_bar(payload),
            );
            write_int(
                &mut patch,
                KEY_SOULS,
                i32::try_from(base.souls).unwrap_or(i32::MAX),
            );
            write_int(
                &mut patch,
                KEY_PROGRESS,
                i32::try_from(base.progress).unwrap_or(i32::MAX),
            );
            write_int(
                &mut patch,
                KEY_BAR,
                i32::try_from(base.bar).unwrap_or(i32::MAX),
            );
        }
        Machine::Pipe => write_int(
            &mut patch,
            KEY_SOULS,
            read_int(payload, KEY_SOULS)
                .max(0)
                .min(i32::try_from(runtime::SOUL_PER_PIPE).unwrap_or(i32::MAX)),
        ),
        Machine::Surge => write_int(
            &mut patch,
            KEY_WAIT,
            read_int(payload, KEY_WAIT)
                .max(0)
                .min(i32::try_from(runtime::SOUL_TIME).unwrap_or(i32::MAX)),
        ),
    }
    Some(patch)
}

/// The type id a payload names.
fn payload_id(payload: &Compound) -> Option<String> {
    match payload.get(&"id".into())? {
        Tag::String(id) if !id.to_string_lossy().is_empty() => Some(id.to_string_lossy()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A base snapshot as the break sees it, with the snapshot's own noise beside it (the
    /// coordinates, the pack flags and a stored `Items` list a mod block entity may keep).
    fn a_base_with_contents() -> Compound {
        let mut nbt = Compound::new();
        write_int(&mut nbt, KEY_SOULS, 42);
        write_int(&mut nbt, KEY_PROGRESS, 7);
        write_int(&mut nbt, KEY_BAR, 16);
        nbt.insert("x".into(), Tag::Int(194));
        nbt.insert("y".into(), Tag::Int(101));
        nbt.insert("z".into(), Tag::Int(0));
        nbt.insert("keepPacked".into(), Tag::Byte(0));
        nbt.insert("Items".into(), Tag::List(Vec::new()));
        nbt
    }

    /// The two stacks a break's stash carries: a lens in slot 0 and the tier-2 addon in slot 1, as
    /// `ItemStack::to_nbt` writes them beside their `Slot`.
    fn a_stash() -> Vec<Tag> {
        let mut lens = Compound::new();
        lens.insert("Slot".into(), Tag::Byte(0));
        lens.insert("id".into(), Tag::String(runtime::LENS.into()));
        let mut addon = Compound::new();
        addon.insert("Slot".into(), Tag::Byte(1));
        addon.insert(
            "id".into(),
            Tag::String("industrialforegoing:efficiency_addon_tier_2".into()),
        );
        vec![Tag::Compound(lens), Tag::Compound(addon)]
    }

    fn payload_of(patch: &Compound) -> Compound {
        match patch.get(&BLOCK_ENTITY_DATA.into()) {
            Some(Tag::Compound(payload)) => payload.clone(),
            _ => panic!("the patch has no {BLOCK_ENTITY_DATA}"),
        }
    }

    /// The family rule of slice 3: a broken machine's item carries what the machine held, as a
    /// **fresh** payload of the keys this package owns — not the pre-removal snapshot.
    #[test]
    fn a_machine_s_item_carries_its_contents() {
        let patch = carried_patch(Machine::Base, &a_base_with_contents(), Some(a_stash()))
            .expect("a machine with a lens, an augment and souls has something to carry");
        let payload = payload_of(&patch);
        assert_eq!(
            payload_id(&payload).as_deref(),
            Some(BASE),
            "vanilla's TypedEntityData loads a payload only into the type it names"
        );
        assert_eq!(read_int(&payload, KEY_SOULS), 42, "the tank travels");
        assert_eq!(
            read_progress(&payload),
            Some(7),
            "the bar's position travels"
        );
        assert_eq!(read_bar(&payload), Some(16), "the bar's length travels");
        let entries = match payload.get(&KEY_CARRIED.into()) {
            Some(Tag::List(entries)) => entries.clone(),
            other => panic!("the payload's carried list is {other:?}"),
        };
        assert_eq!(entries.len(), 2, "both stacks travel");
        assert_eq!(
            read_lens(&payload),
            None,
            "the old `lens` key is not written any more: the stack is the record"
        );
        for key in ["x", "y", "z", "keepPacked", "Items", "components"] {
            assert!(
                !payload.contains_key(&key.into()),
                "{key} belongs to the old position, not to the item"
            );
        }
    }

    /// The counterfactual: a machine at rest keeps nothing, so its item stays plain.
    #[test]
    fn an_empty_machine_carries_nothing() {
        let mut empty = Compound::new();
        write_int(&mut empty, KEY_SOULS, 0);
        write_int(&mut empty, KEY_PROGRESS, 20);
        write_int(&mut empty, KEY_BAR, 20);
        assert_eq!(carried_patch(Machine::Base, &empty, None), None);
        let mut pipe = Compound::new();
        write_int(&mut pipe, KEY_SOULS, 0);
        assert_eq!(carried_patch(Machine::Pipe, &pipe, None), None);
        let mut surge = Compound::new();
        write_int(&mut surge, KEY_WAIT, 0);
        assert_eq!(carried_patch(Machine::Surge, &surge, None), None);
    }

    /// A pipe's own souls and a surge's bought ticks travel too.
    #[test]
    fn a_pipe_carries_its_souls_and_a_surge_its_ticks() {
        let mut pipe = Compound::new();
        write_int(&mut pipe, KEY_SOULS, 3);
        let payload =
            payload_of(&carried_patch(Machine::Pipe, &pipe, None).expect("three souls travel"));
        assert_eq!(payload_id(&payload).as_deref(), Some(PIPE));
        assert_eq!(read_int(&payload, KEY_SOULS), 3);

        let mut surge = Compound::new();
        write_int(&mut surge, KEY_WAIT, 120);
        let payload =
            payload_of(&carried_patch(Machine::Surge, &surge, None).expect("its ticks travel"));
        assert_eq!(payload_id(&payload).as_deref(), Some(SURGE));
        assert_eq!(read_int(&payload, KEY_WAIT), 120);
    }

    /// Placing a carried item puts the keys back, clamped: a hand-written payload or another
    /// version's save may carry anything, as upstream's components clamp on load.
    #[test]
    fn a_carried_payload_is_clamped_to_the_capacities() {
        let mut payload = Compound::new();
        payload.insert("id".into(), Tag::String(BASE.into()));
        write_int(&mut payload, KEY_SOULS, 99_999);
        write_int(&mut payload, KEY_PROGRESS, 500);
        write_int(&mut payload, KEY_BAR, 0);
        let patch = restore_patch(Machine::Base, &payload).expect("a base payload restores");
        let base = LaserBase::from_saved(
            read_int(&patch, KEY_SOULS),
            read_progress(&patch),
            read_bar(&patch),
        );
        assert_eq!(base.souls, SOUL_STORAGE, "the tank stops at 1,350");
        assert_eq!(base.bar, MAX_PROGRESS, "a zero bar is no bar");
        assert_eq!(
            base.progress, base.bar,
            "a position past the bar is the bar"
        );

        let mut pipe = Compound::new();
        pipe.insert("id".into(), Tag::String(PIPE.into()));
        write_int(&mut pipe, KEY_SOULS, 99);
        let patch = restore_patch(Machine::Pipe, &pipe).expect("a pipe payload restores");
        assert_eq!(
            read_int(&patch, KEY_SOULS),
            i32::try_from(runtime::SOUL_PER_PIPE).unwrap_or(4),
            "a pipe holds four souls"
        );

        let mut surge = Compound::new();
        surge.insert("id".into(), Tag::String(SURGE.into()));
        write_int(&mut surge, KEY_WAIT, 9_999);
        let patch = restore_patch(Machine::Surge, &surge).expect("a surge payload restores");
        assert_eq!(
            read_int(&patch, KEY_WAIT),
            i32::try_from(runtime::SOUL_TIME).unwrap_or(300),
            "one soul buys 300 ticks, no more"
        );
    }

    /// The payload names its own type: a base's payload never reaches a pipe's block entity.
    #[test]
    fn a_payload_of_another_machine_is_not_restored() {
        let mut payload = Compound::new();
        payload.insert("id".into(), Tag::String(BASE.into()));
        write_int(&mut payload, KEY_SOULS, 5);
        assert!(restore_patch(Machine::Base, &payload).is_some());
        assert_eq!(restore_patch(Machine::Pipe, &payload), None);
        assert_eq!(restore_patch(Machine::Surge, &payload), None);

        let mut untyped = Compound::new();
        write_int(&mut untyped, KEY_SOULS, 5);
        assert_eq!(restore_patch(Machine::Base, &untyped), None);
    }

    /// A payload's `carried` list is the container's stacks: positionally, one item a slot, and a
    /// slot index past the machine's three is dropped. (The speed/duplicate augment rule is
    /// `runtime::slot_accepts`, which the click, the menu walk and the restore all ask;
    /// `runtime.rs` pins it.)
    #[test]
    fn a_carried_payload_restores_one_item_a_slot() {
        let entry = |slot: i8, id: &str| {
            let mut stack = Compound::new();
            stack.insert("Slot".into(), Tag::Byte(slot));
            stack.insert("id".into(), Tag::String(id.into()));
            Tag::Compound(stack)
        };
        let mut payload = Compound::new();
        payload.insert(
            KEY_CARRIED.into(),
            Tag::List(vec![
                entry(0, runtime::LENS),
                entry(1, "industrialforegoing:efficiency_addon_tier_2"),
                entry(7, runtime::LENS),
            ]),
        );
        let slots: Vec<i64> = match payload.get(&KEY_CARRIED.into()) {
            Some(Tag::List(entries)) => entries
                .iter()
                .filter_map(|entry| match entry {
                    Tag::Compound(entry) => match entry.get(&"Slot".into()) {
                        Some(Tag::Byte(slot)) => Some(i64::from(*slot)),
                        _ => None,
                    },
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        assert_eq!(slots, vec![0, 1, 7]);
        assert_eq!(
            slots
                .iter()
                .filter(|slot| **slot < SLOT_COUNT as i64)
                .count(),
            2,
            "two of the three entries land in the machine's own slots"
        );
    }

    /// The engine's own refusal path: `install_parts` answers an error when one Part type is
    /// attached to one object twice, and the same message the parent port pins. The three item
    /// Parts must be accepted.
    #[test]
    fn the_engine_accepts_every_part_of_this_package() {
        let category = |_: &str| false;
        let position = |_: &str| None;
        let schema = mod_api::__private::RuleSchema {
            category: &category,
            position: &position,
            rule_count: 0,
        };
        let mods = vec![mod_api::ModEntry::new(
            "industrialforegoingsouls",
            Box::new(Mod),
        )];
        let refused =
            mod_api::__private::install_parts(&mods, &schema, |_: &mod_api::DimensionSettings| {
                false
            });
        assert!(
            refused.is_ok(),
            "the engine refuses this package: {refused:?}"
        );
    }
}
