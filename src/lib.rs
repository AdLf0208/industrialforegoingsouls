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
//!
//! # The harness commands
//!
//! Upstream registers no commands. Two of the three blocks have no GUI in this port (Titanium's
//! screens are the parent's), so the numbers are read and set through
//! `/industrialforegoingsouls` — the precedent is `/powah info|set` and the parent port's own
//! `/industrialforegoing info`. `info` is for everyone; `souls` and `wait` are operator commands.
pub mod runtime;

use std::collections::VecDeque;
use std::sync::OnceLock;

use mod_api::{
    BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent, BlockEntityTypeKey, BlockPart,
    BlockState, Command, CommandArgumentValue, CommandInvocation, Compound, Direction, EntityRef,
    Flow, Hand, Hooks, Host, Identifier, InteractionResult, ItemKey, ItemStack, LevelHost,
    LevelRef, RandomSource, RegisterCommandsEvent, Registrar, RegistrationContext,
    RunModCommandEvent, ServerHost, StateForPlacementEvent, Tag, Text, UseItemOnEvent,
    UseWithoutItemEvent,
};

use crate::runtime::{
    LaserBase, MAX_PROGRESS, Network, SOUL_STORAGE, Surge, Target, lens_fits, pipe_pull,
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
/// Titanium's augment inventory: two slots, kept by position.
const AUGMENT_SLOTS: usize = 2;
const KEY_AUGMENTS: &str = "augments";
/// The item the seated augment came from, so an empty hand can take it back: this port ships no
/// GUI, so the click is the slot's only exit as well as its only entrance.
const KEY_AUGMENT_ITEM: &str = "augment_item";
/// The surge's saved key: the accelerated ticks it still owns.
const KEY_WAIT: &str = "wait";
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

/// The mod has no settings: everything it keeps is in the block entities' saved compounds.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the three blocks' block entity types and the Parts that tick them.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let _ = r.block_key(BASE);
        let _ = r.block_key(PIPE);
        let _ = r.block_key(SURGE);
        let _ = r.item_key(BASE);
        let _ = r.item_key(PIPE);
        let _ = r.item_key(SURGE);
        let _ = LENS_ITEM.set(r.item_key(runtime::LENS));
        let _ = WARDEN_KIND.set(r.entity_type_key(WARDEN));
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
        // The keys are kept for the command path, which must prove a position holds one of this
        // mod's block entities without swapping a level.
        let _ = (
            BASE_KIND.set(base_kind),
            PIPE_KIND.set(pipe_kind),
            SURGE_KIND.set(surge_kind),
        );
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
    /// Right-clicking the machine with the blue lens seats it, taking one out of the hand; a stack
    /// carrying Titanium's `titanium:augments` component seats an augment the same way. Upstream's
    /// slots are Titanium's, opened by a GUI this port does not ship; the click is the same seat
    /// without a screen.
    ///
    /// `canAcceptAugment` refuses a **speed** augment on this machine (it has nothing to speed up:
    /// the bar is the harvest's only clock) and refuses a second augment of a type already seated;
    /// both answers leave the stack in the hand, exactly as the GUI's slot would. A stack with no
    /// augment component and no lens passes through.
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
        if stack.item() == *LENS_ITEM.get()? {
            let seated = LENS_ITEM.get()?.id(&*host);
            let mut patch = Compound::new();
            patch.insert(KEY_LENS.into(), Tag::String(seated.as_str().into()));
            entity.merge_data(host, patch).ok()?;
            shrink_hand(host, click.player, click.hand, stack)?;
            return Some(InteractionResult::Consume);
        }
        let component = augment_component(&*host, stack)?;
        if has_speed(&component) || efficiency_of(&component).is_none() {
            return None;
        }
        let saved = entity.data(&*host)?;
        let mut slots = augment_slots(&saved);
        if slots
            .iter()
            .flatten()
            .any(|slot| efficiency_of(slot).is_some())
        {
            return None;
        }
        let index = slots.iter().position(Option::is_none)?;
        slots[index] = Some(component);
        let mut patch = Compound::new();
        write_augment_slots(&mut patch, &slots);
        let item = stack.item().id(&*host);
        patch.insert(KEY_AUGMENT_ITEM.into(), Tag::String(item.as_str().into()));
        entity.merge_data(host, patch).ok()?;
        shrink_hand(host, click.player, click.hand, stack)?;
        Some(InteractionResult::Consume)
    }

    /// An empty hand takes the lens back out — the only way a survival player can change colours,
    /// since the slot has no screen here — and, when no lens is seated, the augment. The hand is
    /// empty (the hook runs for one), so what comes back goes into the selected hotbar slot.
    ///
    /// The engine also reaches this hook on a right-click whose item use passed, with the stack
    /// **still in the hand** (measured 2026-11-22: a refused speed addon's click took the seated
    /// efficiency addon back instead of doing nothing), so the hand is read first and a held stack
    /// means this is not the empty-hand path.
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
        let player = host.player(click.player)?;
        let slot = match click.hand {
            Hand::Main => u32::try_from(player.selected_slot(&*host)?).ok()?,
            Hand::Off => 40,
        };
        if player
            .inventory_slots(&*host)
            .is_some_and(|slots| slots.iter().any(|(held, _)| *held == slot))
        {
            return None;
        }
        let saved = entity.data(&*host)?;
        if let Some(seated) = read_lens(&saved) {
            let item = host.item_key(&Identifier::parse(&seated).ok()?)?;
            let stack = ItemStack::new(item, 1)?;
            if player.set_inventory_slot(host, slot, Some(stack)).is_err() {
                return None;
            }
            let mut rest = saved;
            rest.remove(&KEY_LENS.into());
            entity.replace_data(host, rest).ok()?;
            return Some(InteractionResult::Consume);
        }
        let seated = read_augment_item(&saved)?;
        let item = host.item_key(&Identifier::parse(&seated).ok()?)?;
        let stack = ItemStack::new(item, 1)?;
        if player.set_inventory_slot(host, slot, Some(stack)).is_err() {
            return None;
        }
        let mut rest = saved;
        rest.remove(&KEY_AUGMENTS.into());
        rest.remove(&KEY_AUGMENT_ITEM.into());
        entity.replace_data(host, rest).ok()?;
        Some(InteractionResult::Consume)
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Laser {
    /// The machine ticks on the server (`EntityBlock.getTicker`).
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: walk the bar, and on its last tick wound the warden this machine covers and
    /// store the souls of the wound. The bar's length is the seated efficiency augment's rescale.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        let saved = event.entity.nbt.clone();
        let lens = read_lens(&saved).is_some_and(|item| lens_fits(&item));
        let mut base = LaserBase::from_saved(
            read_int(&saved, KEY_SOULS),
            read_progress(&saved),
            read_bar(&saved),
        );
        let warden = warden_near(host, level, event.pos);
        let health = warden.and_then(|entity| living_health(&*host, entity));
        if base.tick(lens, health, seated_efficiency(&saved))
            && let Some(entity) = warden
        {
            let _ = entity.hurt(host, GENERIC, runtime::DAMAGE_PER_OPERATION, None, None);
        }
        if read_int(&saved, KEY_SOULS) != i32::try_from(base.souls).unwrap_or(i32::MAX) {
            write_int(
                &mut event.entity.nbt,
                KEY_SOULS,
                i32::try_from(base.souls).unwrap_or(0),
            );
        }
        if read_progress(&saved)
            != Some(i32::try_from(base.progress).unwrap_or(MAX_PROGRESS as i32))
        {
            write_int(
                &mut event.entity.nbt,
                KEY_PROGRESS,
                i32::try_from(base.progress).unwrap_or(0),
            );
        }
        if read_bar(&saved) != Some(i32::try_from(base.bar).unwrap_or(i32::MAX)) {
            write_int(
                &mut event.entity.nbt,
                KEY_BAR,
                i32::try_from(base.bar).unwrap_or(0),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The Soul Pipe
// ---------------------------------------------------------------------------

/// The pipe's Part: one network transfer a tick.
struct Pipe;

#[mod_api::horizon_part]
impl BlockPart for Pipe {}

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
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        let own = read_int(&event.entity.nbt, KEY_SOULS).max(0);
        let Some(pipe) = host.block_key(PIPE) else {
            return;
        };
        let (pipes, mut network) = walk(host, level, event.pos, Some((event.pos, own as u32)));
        let mut distributed = false;
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
            let taken = pipe_pull(offered, &mut network);
            if taken > 0 && drain_handler(host, level, target, taken) {
                distributed = true;
            }
        }
        if distributed {
            write_network(
                host,
                level,
                &pipes,
                &network,
                Some(event.pos),
                &mut event.entity.nbt,
            );
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
// The network walk
// ---------------------------------------------------------------------------

/// The pipes connected to `origin`, and what each holds.
///
/// Upstream's network is Titanium's `block_network` graph of elements; here it is the connected
/// component of pipe blocks, walked over the six sides. `own` supplies the souls of the pipe whose
/// own tick is asking — a block entity out for its tick cannot be read — and `None` when the walk
/// starts at a surge.
fn walk(
    host: &mut dyn ServerHost,
    level: LevelRef,
    origin: [i32; 3],
    own: Option<([i32; 3], u32)>,
) -> (Vec<[i32; 3]>, Network) {
    let Some(pipe) = host.block_key(PIPE) else {
        return (Vec::new(), Network::new([]));
    };
    let mut positions: Vec<[i32; 3]> = Vec::new();
    let mut souls: Vec<u32> = Vec::new();
    let mut queue: VecDeque<[i32; 3]> = VecDeque::new();
    match own {
        Some((pos, amount)) => {
            positions.push(pos);
            souls.push(amount);
            queue.push_back(pos);
        }
        None => {
            for direction in Direction::ALL {
                let offset = direction.normal();
                let next = [
                    origin[0] + offset[0],
                    origin[1] + offset[1],
                    origin[2] + offset[2],
                ];
                if level.block(host, next) == Some(pipe) && !positions.contains(&next) {
                    positions.push(next);
                    souls.push(pipe_souls(host, level, next));
                    queue.push_back(next);
                }
            }
        }
    }
    while let Some(pos) = queue.pop_front() {
        for direction in Direction::ALL {
            let offset = direction.normal();
            let next = [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]];
            if positions.contains(&next) || level.block(host, next) != Some(pipe) {
                continue;
            }
            positions.push(next);
            souls.push(pipe_souls(host, level, next));
            queue.push_back(next);
        }
    }
    (positions, Network::new(souls))
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
            continue;
        }
        let Some(entity) = host.block_entity(level, *pos) else {
            continue;
        };
        let mut patch = Compound::new();
        write_int(&mut patch, KEY_SOULS, amount);
        let _ = entity.merge_data(host, patch);
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

/// The two slots as the machine saved them, by position: an empty slot is an empty compound, so a
/// second augment never slides into the first one's place.
fn augment_slots(nbt: &Compound) -> [Option<Compound>; AUGMENT_SLOTS] {
    let mut slots = [const { None }; AUGMENT_SLOTS];
    let Some(Tag::List(entries)) = nbt.get(&KEY_AUGMENTS.into()) else {
        return slots;
    };
    for (slot, entry) in slots.iter_mut().zip(entries) {
        if let Tag::Compound(component) = entry
            && !component.is_empty()
        {
            *slot = Some(component.clone());
        }
    }
    slots
}

/// Write both slots back, keeping the positions.
fn write_augment_slots(nbt: &mut Compound, slots: &[Option<Compound>; AUGMENT_SLOTS]) {
    let entries = slots
        .iter()
        .map(|slot| Tag::Compound(slot.clone().unwrap_or_default()))
        .collect();
    nbt.insert(KEY_AUGMENTS.into(), Tag::List(entries));
}

/// The efficiency augment in force, `None` when no slot holds one (the plain 20-tick bar).
fn seated_efficiency(nbt: &Compound) -> Option<f32> {
    augment_slots(nbt).iter().flatten().find_map(efficiency_of)
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
            let lens = read_lens(&data).unwrap_or_else(|| "none".to_owned());
            let efficiency = seated_efficiency(&data).unwrap_or(1.0);
            let warden =
                warden_near(host, level, pos).and_then(|entity| living_health(&*host, entity));
            let warden = match warden {
                Some(health) => format!("{health:.1}"),
                None => "none".to_owned(),
            };
            format!(
                "Soul Laser Base at {pos:?}: souls {}/{SOUL_STORAGE}, progress {}/{}, lens {lens}, augments efficiency {efficiency:.2}, warden {warden}, t={time}",
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
            format!(
                "Soul Pipe at {pos:?}: network {}/{}, t={time}",
                network.total(),
                network.capacity()
            )
        }
        Some(kind) if Some(kind) == SURGE_KIND.get().copied() => {
            let (_, network) = walk(host, level, pos, None);
            let surge = Surge::from_saved(read_int(&data, KEY_WAIT));
            let facing = level.block_property(host, pos, FACING).unwrap_or_default();
            let powered = level.has_neighbor_signal(&*host, pos);
            format!(
                "Soul Surge at {pos:?}: network {}/{}, wait {}/{}, facing {facing}, powered {powered}, t={time}",
                network.total(),
                network.capacity(),
                surge.ticking_time,
                runtime::SOUL_TIME
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

/// Take one item out of the hand that clicked, leaving the rest of the stack.
fn shrink_hand(
    host: &mut dyn ServerHost,
    player: u128,
    hand: Hand,
    stack: &ItemStack,
) -> Option<()> {
    let player = host.player(player)?;
    let slot = match hand {
        Hand::Main => player.selected_slot(&*host)? as u32,
        Hand::Off => 40,
    };
    player
        .set_inventory_slot(host, slot, stack.clone().shrink(1))
        .ok()
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

/// The item id the seated augment came from, for the empty hand's take-back.
fn read_augment_item(nbt: &Compound) -> Option<String> {
    match nbt.get(&KEY_AUGMENT_ITEM.into())? {
        Tag::String(item) if !item.to_string_lossy().is_empty() => Some(item.to_string_lossy()),
        _ => None,
    }
}
