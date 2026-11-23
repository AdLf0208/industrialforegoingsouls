//! Industrial Foregoing: Souls — the soul economy as arithmetic.
//!
//! Ported from `Industrial-Foregoing-Souls` 1.21.1-1.10.7 (MIT). The mod is one mechanic in three
//! blocks: the **Soul Laser Base** turns a warden's health into souls, the **Soul Pipe** carries
//! them along a network, and the **Soul Surge** spends them to random-tick a block. Every number
//! below is the jar's own config default (`ConfigSoulLaserBase`, `ConfigSoulSurge`,
//! `IFSoulsMachines`), and nothing here touches the game: the glue in `lib.rs` reads a block
//! entity, calls one of these, and writes the result back.
//!
//! What is deliberately *not* a config value: this port has no config file, so a server owner
//! cannot retune the economy without a new build. That is a recorded deviation, not an oversight.

/// Souls the laser base's tank holds (`ConfigSoulLaserBase.SOUL_STORAGE_AMOUNT`).
pub const SOUL_STORAGE: u32 = 1350;
/// Ticks between two laser-base operations (`ConfigSoulLaserBase.MAX_PROGRESS`).
pub const MAX_PROGRESS: u32 = 20;
/// Souls one operation stores (`ConfigSoulLaserBase.SOULS_PER_OPERATION`).
pub const SOULS_PER_OPERATION: u32 = 2;
/// Health an operation takes off the warden (`ConfigSoulLaserBase.DAMAGE_PER_OPERATION`). The
/// operation only fires while the warden's health is **above** this, which is what keeps the mod's
/// one soul source alive (`KILL_WARDEN` is false upstream, and this port does not offer it).
pub const DAMAGE_PER_OPERATION: f32 = 4.0;
/// Souls one pipe adds to its network's capacity (`IFSoulsMachines.SOUL_AMOUNT_PER_PIPE`).
pub const SOUL_PER_PIPE: u32 = 4;
/// Souls a pipe moves between a handler and its network in one tick — the hard-coded `4` in
/// `SoulPipeBlockEntity.serverTick`, not a config value upstream either.
pub const HANDLER_TRANSFER: u32 = 4;
/// Ticks one soul buys the surge (`ConfigSoulSurge.SOUL_TIME`, `15 * 20`).
pub const SOUL_TIME: u32 = 300;
/// Random ticks each acceleration applies (`ConfigSoulSurge.BLOCK_ACCELERATION_TICK`).
pub const BLOCK_ACCELERATION_TICK: u32 = 4;
/// Chance per tick that a surge's facing block is accelerated (`RANDOM_TICK_ACCELERATION_CHANCE`).
pub const RANDOM_TICK_ACCELERATION_CHANCE: f64 = 0.03;
/// The one item a laser base reads in its catalyst slot: `ModuleCore.LASER_LENS[11]`, the blue lens
/// of the 16 the parent mod registers (`DyeColor.BLUE.ordinal()`).
pub const LENS: &str = "industrialforegoing:blue_laser_lens";

/// The bar a machine with an efficiency augment runs: `(int) Math.floor(MAX_PROGRESS * efficiency)`,
/// the rescale `SoulLaserBaseBlockEntity.onWork` ends with. A value that would give a bar of zero
/// (or less than one tick) is clamped to one, because a zero bar never finishes; the two augments
/// upstream ships are 0.9 (tier 1) and 0.8 (tier 2).
#[must_use]
pub fn bar_max(efficiency: f32) -> u32 {
    let ticks = (MAX_PROGRESS as f32 * efficiency).floor();
    // A negative, NaN or absurd value saturates rather than wrapping, and a bar of zero would
    // never finish, so one tick is the floor.
    u32::try_from(ticks as i64).unwrap_or(1).max(1)
}

/// The laser base's own state: the soul tank and the operation bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LaserBase {
    /// Souls in the 1,350 tank.
    pub souls: u32,
    /// Ticks left on the bar; 0 fires on the next tick.
    pub progress: u32,
    /// Ticks one full bar is: [`MAX_PROGRESS`], or the efficiency augment's rescale.
    pub bar: u32,
}

impl LaserBase {
    /// A machine from its saved numbers, clamped to the two capacities (a `/data merge` or an older
    /// save may carry anything; a machine that starts a bar from nothing gets a full one). A saved
    /// bar of zero is not a bar, so it is read as [`MAX_PROGRESS`].
    #[must_use]
    pub fn from_saved(souls: i32, progress: Option<i32>, bar: Option<i32>) -> Self {
        let souls = u32::try_from(souls).unwrap_or(0).min(SOUL_STORAGE);
        let bar = bar
            .and_then(|ticks| u32::try_from(ticks).ok())
            .filter(|ticks| *ticks > 0)
            .unwrap_or(MAX_PROGRESS);
        let progress = progress
            .and_then(|ticks| u32::try_from(ticks).ok())
            .unwrap_or(bar)
            .min(bar);
        Self {
            souls,
            progress,
            bar,
        }
    }

    /// One tick of the bar. Returns whether the operation fired, in which case the caller hurts the
    /// warden it found by [`DAMAGE_PER_OPERATION`]; the souls are already in [`LaserBase::souls`].
    ///
    /// The operation needs the blue lens seated, a tank with room, and a warden whose health is
    /// above [`DAMAGE_PER_OPERATION`]. The bar resets whether or not it fired, as the parent's
    /// `ProgressBarComponent` with `setCanReset(true)` does — and, as upstream's `onWork` does at
    /// its end, the reset length is the augmented one, so the *next* cycle is the short one.
    /// `efficiency` is the seated efficiency augment's value, `None` when there is none.
    pub fn tick(
        &mut self,
        lens: bool,
        warden_health: Option<f32>,
        efficiency: Option<f32>,
    ) -> bool {
        self.progress = self.progress.saturating_sub(1);
        if self.progress > 0 {
            return false;
        }
        self.bar = efficiency.map_or(MAX_PROGRESS, bar_max);
        self.progress = self.bar;
        let has_room = self.souls < SOUL_STORAGE;
        let woundable = warden_health.is_some_and(|health| health > DAMAGE_PER_OPERATION);
        if !(lens && has_room && woundable) {
            return false;
        }
        self.souls = (self.souls + SOULS_PER_OPERATION).min(SOUL_STORAGE);
        true
    }

    /// `ComparatorBlock`'s reading of the tank: `floor(15 * souls / 1350)`.
    #[must_use]
    pub fn analog_output(&self) -> i32 {
        i32::try_from(15 * self.souls / SOUL_STORAGE).unwrap_or(15)
    }
}

/// Whether a seated item is the lens this machine reads. Upstream compares the item against
/// `ModuleCore.LASER_LENS[11]`; the other fifteen colours are inert here.
#[must_use]
pub fn lens_fits(item: &str) -> bool {
    item == LENS
}

/// The most pipes one network walk holds. A walk stops at this budget, so a corrupted or
/// deliberately huge arrangement cannot make one tick unbounded; the tail beyond it joins the
/// network again once the component shrinks.
pub const MAX_NETWORK_PIPES: usize = 4096;

// -----------------------------------------------------------------------
// Waterlogging (`SimpleWaterloggedBlock`, slice 0.2.3)
// -----------------------------------------------------------------------

/// The state property a waterlogged block of this mod declares. Upstream's `SimpleWaterloggedBlock`
/// spells it `waterlogged`, and the engine reads a state's water from this *name* (see `notes.md`).
pub const WATERLOGGED: &str = "waterlogged";
/// A full bucket, the item whose click fills a dry machine
/// (`BucketItem.emptyContents` against a liquid container).
pub const WATER_BUCKET: &str = "minecraft:water_bucket";
/// An empty bucket, the item whose click drains a waterlogged one (`BucketPickup.pickupBlock`).
pub const BUCKET: &str = "minecraft:bucket";

/// Whether a machine of this mod holds the water in its own state, as vanilla's `WATERLOGGED`
/// boolean does. A typed value rather than a `bool`, so a call site reads as the state it writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Waterlogged {
    /// Nothing in the state: the dry block.
    #[default]
    Dry,
    /// The block holds a water source.
    Holding,
}

impl Waterlogged {
    /// The state's saved value: only `true` is waterlogged, exactly as vanilla reads its own
    /// boolean (`getValue(WATERLOGGED)`); a missing or unknown value is dry.
    #[must_use]
    pub fn from_property(value: Option<&str>) -> Self {
        match value {
            Some("true") => Self::Holding,
            _ => Self::Dry,
        }
    }

    /// The value the property is written with.
    #[must_use]
    pub fn property(self) -> &'static str {
        match self {
            Self::Dry => "false",
            Self::Holding => "true",
        }
    }

    /// Whether the block holds water.
    #[must_use]
    pub fn holds_water(self) -> bool {
        self == Self::Holding
    }
}

/// What one bucket click does to a machine of this mod, worked out from the item and the state
/// **before** anything is written, so a refused click changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterClick {
    /// A full bucket on a dry machine: the water goes into the state.
    Fill,
    /// An empty bucket on a waterlogged machine: the water comes back out.
    Drain,
}

/// `SimpleWaterloggedBlock.canPlaceLiquid`/`pickupBlock` as a click: a water bucket fills a **dry**
/// machine (vanilla refuses one that is already waterlogged, `!state.getValue(WATERLOGGED)`), an
/// empty bucket drains a **waterlogged** one (a dry one gives nothing), and every other item is no
/// water click at all.
#[must_use]
pub fn water_click(item: &str, state: Waterlogged) -> Option<WaterClick> {
    match (item, state) {
        (WATER_BUCKET, Waterlogged::Dry) => Some(WaterClick::Fill),
        (BUCKET, Waterlogged::Holding) => Some(WaterClick::Drain),
        _ => None,
    }
}

/// The slot an empty-bucket drain hands its full bucket into: the first free hotbar or
/// main-inventory slot that is not the clicking hand, then the off hand; `None` when the inventory
/// is full, so the drain is refused rather than throwing the water away.
///
/// Vanilla's `ItemUtils.createFilledResult` puts the filled bucket **in the hand** the empty one
/// leaves. This pin reverts a water bucket written into the held slot from a block's `use_item_on`
/// (measured 2026-11-23: the slot holds it inside the hook and an empty bucket again a few ticks
/// later, through both `set_inventory_slot` and `give_stack`, while the same write to a main
/// inventory slot persists and a plain item in the held slot persists too). So the water goes to
/// the inventory instead, and `notes.md` records the difference.
#[must_use]
pub fn bucket_destination(occupied: &[u32], hand_slot: u32) -> Option<u32> {
    (0..=35)
        .chain(std::iter::once(40))
        .find(|slot| *slot != hand_slot && !occupied.contains(slot))
}

/// The slot a replaced catalyst lens goes back into: the clicking hand's own slot when it holds a
/// single item (it empties), otherwise the first empty hotbar or main-inventory slot, then the off
/// hand. `None` when the inventory is full, so the click is refused instead of voiding the lens.
/// `occupied` is [`mod_api::PlayerRef::inventory_slots`]'s numbers, and `hand_count` the clicking
/// stack's count before one is taken.
#[must_use]
pub fn replacement_slot(occupied: &[u32], hand_slot: u32, hand_count: u32) -> Option<u32> {
    if hand_count <= 1 {
        return Some(hand_slot);
    }
    // `Inventory`'s numbers: 0..=8 the hotbar, 9..=35 the main inventory, 40 the off hand.
    (0..=35)
        .chain(std::iter::once(40))
        .find(|slot| *slot != hand_slot && !occupied.contains(slot))
}

/// The connected pipe positions grown from `seeds`, each cell probed once, cut at `limit` pipes.
/// Set-based: a neighbour is looked up in the visited set, never in a position list.
#[must_use]
pub fn grow_component(
    seeds: impl IntoIterator<Item = [i32; 3]>,
    limit: usize,
    is_pipe: &mut dyn FnMut([i32; 3]) -> bool,
) -> Vec<[i32; 3]> {
    /// The six sides, in `Direction::ALL` order.
    const SIDES: [[i32; 3]; 6] = [
        [0, -1, 0],
        [0, 1, 0],
        [0, 0, -1],
        [0, 0, 1],
        [-1, 0, 0],
        [1, 0, 0],
    ];
    let mut visited: std::collections::HashSet<[i32; 3]> = std::collections::HashSet::new();
    let mut positions: Vec<[i32; 3]> = Vec::new();
    let mut queue: std::collections::VecDeque<[i32; 3]> = std::collections::VecDeque::new();
    for seed in seeds {
        if visited.insert(seed) && is_pipe(seed) && positions.len() < limit {
            positions.push(seed);
            queue.push_back(seed);
        }
    }
    'walk: while let Some(pos) = queue.pop_front() {
        for side in SIDES {
            if positions.len() >= limit {
                break 'walk;
            }
            let next = [pos[0] + side[0], pos[1] + side[1], pos[2] + side[2]];
            if !visited.insert(next) {
                continue;
            }
            if is_pipe(next) {
                positions.push(next);
                queue.push_back(next);
            }
        }
    }
    positions
}

/// One walked network: the connected pipe positions, in walk order, and what each held.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Walked {
    positions: Vec<[i32; 3]>,
    network: Network,
}

impl Walked {
    /// A component from its positions and the souls at each (clamped to a pipe).
    #[must_use]
    pub fn new(positions: Vec<[i32; 3]>, souls: impl IntoIterator<Item = u32>) -> Self {
        Self {
            positions,
            network: Network::new(souls),
        }
    }

    /// The positions, in walk order.
    #[must_use]
    pub fn positions(&self) -> &[[i32; 3]] {
        &self.positions
    }

    /// What each pipe holds, in walk order.
    #[must_use]
    pub fn network(&self) -> &Network {
        &self.network
    }

    /// The network, for the transfer's write-back.
    pub fn network_mut(&mut self) -> &mut Network {
        &mut self.network
    }

    /// Set the souls of the pipe at `pos` (the pipe whose own tick is asking cannot be read).
    pub fn set_souls(&mut self, pos: [i32; 3], souls: u32) {
        if let Some(index) = self.positions.iter().position(|position| *position == pos) {
            self.network.set(index, souls);
        }
    }
}

/// The per-tick memo of walked components, so the second pipe of a component does not walk it
/// again. The key is the level and the level's game time: a new tick drops everything.
#[derive(Debug, Default)]
pub struct NetworkCache {
    key: Option<u64>,
    components: Vec<Walked>,
    by_position: std::collections::HashMap<[i32; 3], (usize, usize)>,
}

impl NetworkCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a walk at `key`, dropping every component cached for another level or tick.
    pub fn begin(&mut self, key: u64) {
        if self.key != Some(key) {
            self.key = Some(key);
            self.components.clear();
            self.by_position.clear();
        }
    }

    /// The component holding `pos`, when this tick already walked it.
    #[must_use]
    pub fn component(&self, pos: [i32; 3]) -> Option<&Walked> {
        self.by_position
            .get(&pos)
            .map(|(component, _)| &self.components[*component])
    }

    /// The component holding `pos`, for the tick's transfer.
    pub fn component_mut(&mut self, pos: [i32; 3]) -> Option<&mut Walked> {
        let component = self.by_position.get(&pos)?.0;
        self.components.get_mut(component)
    }

    /// Remember a walked component, mapping every one of its positions to it.
    pub fn insert(&mut self, walked: Walked) {
        let component = self.components.len();
        for (index, position) in walked.positions.iter().enumerate() {
            self.by_position.insert(*position, (component, index));
        }
        self.components.push(walked);
    }

    /// Update one pipe's souls in the cached component that holds it.
    pub fn set_souls(&mut self, pos: [i32; 3], souls: u32) {
        if let Some((component, index)) = self.by_position.get(&pos).copied() {
            self.components[component].network.set(index, souls);
        }
    }
}

/// A soul network: one entry per connected pipe, in the walk's order, each holding up to
/// [`SOUL_PER_PIPE`]. Upstream keeps the total on Titanium's network object and caps it at
/// `elements * SOUL_PER_PIPE`; storing it per pipe gives the same capacity and the same totals
/// while surviving a save without a network registry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Network {
    /// Souls in each connected pipe, in walk order.
    pipes: Vec<u32>,
}

impl Network {
    /// A network of `pipes` pipes, each holding the amount at its index (clamped to a pipe).
    #[must_use]
    pub fn new(pipes: impl IntoIterator<Item = u32>) -> Self {
        Self {
            pipes: pipes
                .into_iter()
                .map(|souls| souls.min(SOUL_PER_PIPE))
                .collect(),
        }
    }

    /// Souls the whole network can hold: four a pipe.
    #[must_use]
    pub fn capacity(&self) -> u32 {
        u32::try_from(self.pipes.len())
            .unwrap_or(u32::MAX)
            .saturating_mul(SOUL_PER_PIPE)
    }

    /// Souls the network holds now.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.pipes.iter().sum()
    }

    /// What each pipe holds, in walk order, for the write-back.
    #[must_use]
    pub fn pipes(&self) -> &[u32] {
        &self.pipes
    }

    /// Put up to `amount` souls into the network, filling pipes in walk order; returns what fit.
    pub fn add(&mut self, amount: u32) -> u32 {
        let mut left = amount;
        for souls in &mut self.pipes {
            if left == 0 {
                break;
            }
            let put = SOUL_PER_PIPE.saturating_sub(*souls).min(left);
            *souls += put;
            left -= put;
        }
        amount - left
    }

    /// Set one pipe's souls (clamped), for the pipe whose own tick supplies its unreadable value.
    pub fn set(&mut self, index: usize, souls: u32) {
        if let Some(slot) = self.pipes.get_mut(index) {
            *slot = souls.min(SOUL_PER_PIPE);
        }
    }

    /// Take up to `amount` souls out of the network, emptying pipes in walk order; returns what
    /// came out.
    pub fn drain(&mut self, amount: u32) -> u32 {
        let mut left = amount;
        for souls in &mut self.pipes {
            if left == 0 {
                break;
            }
            let took = (*souls).min(left);
            *souls -= took;
            left -= took;
        }
        amount - left
    }
}

/// One pipe's tick against one adjacent handler: it offers what it holds, the pipe asks for at most
/// [`HANDLER_TRANSFER`], and the network keeps what fits. Returns the souls taken out of the handler.
pub fn pipe_pull(handler_souls: u32, network: &mut Network) -> u32 {
    network.add(handler_souls.min(HANDLER_TRANSFER))
}

/// The block a surge points at, as far as its tick cares. Upstream has three branches; this pin can
/// express one of them (see `notes.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Air with nothing to accelerate, or a block the two `cant_accelerate` tags name: the surge
    /// keeps its soul and burns no time, exactly as upstream's air branch does with no entity in it.
    Nothing,
    /// A plain block: [`BLOCK_ACCELERATION_TICK`] random ticks at [`RANDOM_TICK_ACCELERATION_CHANCE`].
    Block,
}

/// What one tick of a surge did, for the glue to carry out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurgeStep {
    /// Souls taken out of the network this tick (0 or 1).
    pub souls_spent: u32,
    /// Random ticks to apply to the facing block now.
    pub random_ticks: u32,
}

/// The surge's own state: how many of its 300 accelerated ticks are left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Surge {
    /// Ticks of acceleration left; 0 means the next tick must buy one.
    pub ticking_time: u32,
}

impl Surge {
    /// A surge from its saved counter (a negative value is an empty one).
    #[must_use]
    pub fn from_saved(ticking_time: i32) -> Self {
        Self {
            ticking_time: u32::try_from(ticking_time).unwrap_or(0),
        }
    }

    /// One tick: buy a soul's worth of time when there is none left and the network has one, then
    /// spend one tick on `target`. `roll` is the level's next random double, passed in so this stays
    /// pure; the 3% comparison is [`RANDOM_TICK_ACCELERATION_CHANCE`].
    pub fn tick(&mut self, network: &mut Network, target: Target, roll: f64) -> SurgeStep {
        let mut step = SurgeStep::default();
        if self.ticking_time == 0 && network.total() > 0 {
            step.souls_spent = network.drain(1);
            self.ticking_time = SOUL_TIME;
        }
        if self.ticking_time == 0 {
            return step;
        }
        // Air (and a tag the block is in) has nothing to accelerate: upstream does not spend the
        // tick there either, so the soul lasts until the surge is pointed at something.
        if target == Target::Nothing {
            return step;
        }
        if roll < RANDOM_TICK_ACCELERATION_CHANCE {
            step.random_ticks = BLOCK_ACCELERATION_TICK;
        }
        self.ticking_time -= 1;
        step
    }
}

// -----------------------------------------------------------------------
// The menu and its slots (slice 0.2.4)
// -----------------------------------------------------------------------

/// The menu type the content declares (`data/industrialforegoingsouls/horizon/menu/soul_laser_base.json`),
/// which is also the block it belongs to.
pub const MENU: &str = "industrialforegoingsouls:soul_laser_base";

/// The container's slots, in menu order: the catalyst (`SidedInventoryComponent lens`, one item) at
/// 0 and Titanium's two augment slots after it, in the order `AugmentInventory` keeps them.
pub const LENS_SLOT: usize = 0;
/// Titanium's two augment slots, in the machine's own order.
pub const AUGMENT_SLOTS: [usize; 2] = [1, 2];
/// The container's slot count: the catalyst and the two augment slots.
pub const SLOT_COUNT: usize = 3;

/// The menu's data slots, in order: the tank, the bar's progress and the bar's length. Upstream's
/// screen draws the tank and the bar from its `ContainerData`; this port's are the block entity's
/// `data`, in the same order.
pub const DATA_COUNT: usize = 3;

/// What the menu syncs each tick: the tank, the bar's progress and the bar's length, clamped to the
/// 16-bit the data slots travel as (a bigger value would wrap on the wire, as vanilla's shorts do).
#[must_use]
pub fn menu_data(base: &LaserBase) -> [i32; DATA_COUNT] {
    // The wire is 16-bit (vanilla's `ContainerData` is a short): a value past it is clamped, never
    // wrapped, so a menu cannot show a tank as a negative number.
    let wire = |value: u32| i32::try_from(value.min(u32::from(i16::MAX as u16))).unwrap_or(0);
    [wire(base.souls), wire(base.progress), wire(base.bar)]
}

/// What a stack of an augment's component is: `canAcceptAugment` reads `AugmentTypes.SPEED` and
/// `Efficiency` out of the `titanium:augments` component, and nothing else is an augment at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Addon {
    /// A stack carrying `Efficiency` (0.9 for tier 1, 0.8 for tier 2).
    Efficiency,
    /// A stack carrying `Speed`: refused on this machine, which has nothing to speed up.
    Speed,
    /// Neither value: not an augment.
    None,
}

/// Whether an addon may go into an augment slot: efficiency only, and only while **no** augment is
/// seated — `SoulLaserBaseBlockEntity.canAcceptAugment` refuses speed outright and refuses a second
/// augment of a type already seated. A swap (an occupied slot) is a placement too, so it is refused
/// by the same rule and the held addon stays in the hand, as 0.2.3's click already did.
#[must_use]
pub fn augment_fits(addon: Addon, seated: bool) -> bool {
    addon == Addon::Efficiency && !seated
}

/// The augment slot a newly clicked efficiency addon goes to: the first of Titanium's two that is
/// empty, in slot order. `None` when the machine refuses it (see [`augment_fits`]) or both slots are
/// taken.
#[must_use]
pub fn augment_slot(empty: [bool; 2], addon: Addon, seated: bool) -> Option<usize> {
    if !augment_fits(addon, seated) {
        return None;
    }
    empty
        .iter()
        .position(|empty| *empty)
        .map(|index| AUGMENT_SLOTS[index])
}

/// What the machine's menu takes in slot `slot`: the catalyst slot holds only the blue lens
/// (`catalyst.getStackInSlot(0).getItem().equals(ModuleCore.LASER_LENS[11].get())`), an augment slot
/// follows [`augment_fits`], and every other slot is the player's inventory, which is not this
/// machine's to refuse.
#[must_use]
pub fn slot_accepts(slot: usize, lens: bool, addon: Addon, seated: bool) -> bool {
    match slot {
        LENS_SLOT => lens,
        slot if AUGMENT_SLOTS.contains(&slot) => augment_fits(addon, seated),
        _ => true,
    }
}

/// The slots a save from 0.2.x (or a carried payload) is adopted into on its first tick: the old
/// package kept the catalyst item id under `lens` and the augment's item id under `augment_item`
/// (the components under `augments`), and this slice keeps the items themselves in the container.
/// Positional: the lens at 0, the one augment at 1.
#[must_use]
pub fn adopted_items(lens: Option<&str>, augments: [Option<&str>; 2]) -> [Option<String>; 3] {
    let adopt = |item: Option<&str>| item.map(str::to_owned);
    [adopt(lens), adopt(augments[0]), adopt(augments[1])]
}

#[cfg(test)]
mod tests {
    use super::*;

    const WARDEN: Option<f32> = Some(500.0);

    /// Run `ticks` ticks against an unwounded warden and return how many fired.
    fn run(base: &mut LaserBase, lens: bool, warden: Option<f32>, ticks: u32) -> u32 {
        run_with(base, lens, warden, ticks, None)
    }

    /// The same, with an efficiency augment's value (`None` for an unaugmented machine).
    fn run_with(
        base: &mut LaserBase,
        lens: bool,
        warden: Option<f32>,
        ticks: u32,
        efficiency: Option<f32>,
    ) -> u32 {
        (0..ticks)
            .filter(|_| base.tick(lens, warden, efficiency))
            .count() as u32
    }

    #[test]
    fn a_full_bar_needs_twenty_ticks_before_the_first_operation() {
        let mut base = LaserBase::from_saved(0, None, None);
        assert_eq!(base.progress, MAX_PROGRESS, "a new machine starts full");
        assert_eq!(
            run(&mut base, true, WARDEN, 19),
            0,
            "nineteen ticks are not twenty"
        );
        assert_eq!(base.souls, 0);
        assert_eq!(
            run(&mut base, true, WARDEN, 1),
            1,
            "the twentieth tick is the operation"
        );
        assert_eq!(
            base.souls, SOULS_PER_OPERATION,
            "one operation stores two souls"
        );
        assert_eq!(
            base.progress, MAX_PROGRESS,
            "the bar resets after the operation"
        );
    }

    #[test]
    fn without_the_lens_no_souls_are_made() {
        let mut base = LaserBase::from_saved(0, None, None);
        assert_eq!(
            run(&mut base, false, WARDEN, 200),
            0,
            "no lens, no operation"
        );
        assert_eq!(base.souls, 0, "the tank stays empty without a lens");
    }

    #[test]
    fn only_the_blue_lens_fits() {
        assert!(
            lens_fits(LENS),
            "the blue lens is the one the machine reads"
        );
        assert!(!lens_fits("industrialforegoing:red_laser_lens"));
        assert!(!lens_fits("industrialforegoing:white_laser_lens"));
        assert!(!lens_fits("minecraft:paper"));
    }

    #[test]
    fn a_warden_at_the_damage_floor_is_left_alone() {
        let mut base = LaserBase::from_saved(0, None, None);
        // `getHealth() > DAMAGE_PER_OPERATION` is strict: at exactly 4 the operation is refused.
        assert_eq!(run(&mut base, true, Some(DAMAGE_PER_OPERATION), 40), 0);
        assert_eq!(base.souls, 0);
        assert_eq!(
            run(&mut base, true, Some(DAMAGE_PER_OPERATION + 0.5), 20),
            1
        );
        assert_eq!(base.souls, SOULS_PER_OPERATION);
    }

    #[test]
    fn no_warden_means_no_operation_and_a_reset_bar() {
        let mut base = LaserBase::from_saved(0, None, None);
        assert_eq!(run(&mut base, true, None, 20), 0);
        assert_eq!(base.souls, 0);
        assert_eq!(base.progress, MAX_PROGRESS, "the bar resets anyway");
    }

    #[test]
    fn the_tank_stops_at_1350() {
        // `soulAmount < SOUL_STORAGE` is checked before the operation, so one soul short still runs
        // it and clamps; a full tank runs nothing.
        let mut base = LaserBase::from_saved(SOUL_STORAGE as i32 - 1, None, None);
        assert_eq!(
            run(&mut base, true, WARDEN, 20),
            1,
            "one soul short still fires"
        );
        assert_eq!(base.souls, SOUL_STORAGE, "the machine does not overfill");
        let mut full = LaserBase::from_saved(SOUL_STORAGE as i32, None, None);
        assert_eq!(
            run(&mut full, true, WARDEN, 60),
            0,
            "a full tank runs nothing"
        );
        assert_eq!(full.souls, SOUL_STORAGE);
    }

    #[test]
    fn a_saved_tank_is_clamped_to_the_capacity() {
        assert_eq!(
            LaserBase::from_saved(9_999, Some(0), None).souls,
            SOUL_STORAGE
        );
        assert_eq!(LaserBase::from_saved(-5, Some(0), None).souls, 0);
        assert_eq!(
            LaserBase::from_saved(10, Some(999), None).progress,
            MAX_PROGRESS
        );
        assert_eq!(LaserBase::from_saved(10, Some(3), None).progress, 3);
        assert_eq!(LaserBase::from_saved(10, Some(0), None).progress, 0);
    }

    #[test]
    fn the_analog_output_scales_the_tank_to_fifteen() {
        assert_eq!(LaserBase::from_saved(0, None, None).analog_output(), 0);
        assert_eq!(
            LaserBase::from_saved(SOUL_STORAGE as i32, None, None).analog_output(),
            15
        );
        assert_eq!(LaserBase::from_saved(675, None, None).analog_output(), 7);
        assert_eq!(LaserBase::from_saved(1349, None, None).analog_output(), 14);
    }

    // -----------------------------------------------------------------------
    // The efficiency augment (`titanium:augments`, Titanium's `AugmentTypes`)
    // -----------------------------------------------------------------------

    /// `SoulLaserBaseBlockEntity.onWork`'s last line: `(int) Math.floor(20 * efficiency)`.
    #[test]
    fn the_bar_maximum_is_the_floor_of_twenty_times_efficiency() {
        assert_eq!(bar_max(0.8), 16, "the tier 2 augment");
        assert_eq!(bar_max(0.9), 18, "the tier 1 augment");
        assert_eq!(bar_max(1.0), MAX_PROGRESS, "an augment of one is no change");
        assert_eq!(bar_max(1.5), 30, "a longer bar is allowed");
        assert_eq!(bar_max(0.0), 1, "a zero bar would never finish");
        assert_eq!(bar_max(f32::NAN), 1, "a bar the arithmetic cannot read");
    }

    /// The first bar is the plain twenty: upstream rescales it at the *end* of `onWork`, so the
    /// operation that seats the value is still the long one.
    #[test]
    fn an_efficiency_augment_shortens_the_bar_after_the_first_operation() {
        let mut base = LaserBase::from_saved(0, None, None);
        assert_eq!(base.bar, MAX_PROGRESS, "an unaugmented machine's bar");
        assert_eq!(
            run_with(&mut base, true, WARDEN, 19, Some(0.8)),
            0,
            "nineteen ticks are not the first twenty"
        );
        assert_eq!(
            run_with(&mut base, true, WARDEN, 1, Some(0.8)),
            1,
            "the first operation is the plain bar's"
        );
        assert_eq!(base.bar, 16, "floor(20 x 0.8) after the operation");
        assert_eq!(
            base.progress, 16,
            "the next cycle starts at the shortened bar"
        );
        assert_eq!(
            run_with(&mut base, true, WARDEN, 15, Some(0.8)),
            0,
            "fifteen of sixteen is not an operation"
        );
        assert_eq!(
            run_with(&mut base, true, WARDEN, 1, Some(0.8)),
            1,
            "the sixteenth is"
        );
        assert_eq!(base.souls, 2 * SOULS_PER_OPERATION);
    }

    #[test]
    fn a_tier_one_efficiency_augment_gives_an_eighteen_tick_bar() {
        let mut base = LaserBase::from_saved(0, None, None);
        assert_eq!(run_with(&mut base, true, WARDEN, 20, Some(0.9)), 1);
        assert_eq!(base.bar, 18, "floor(20 x 0.9)");
        assert_eq!(run_with(&mut base, true, WARDEN, 17, Some(0.9)), 0);
        assert_eq!(run_with(&mut base, true, WARDEN, 1, Some(0.9)), 1);
    }

    /// The parsed number the e2e reads: a shortened bar turns 160 ticks into ten operations where
    /// the plain bar could only fit eight.
    #[test]
    fn ten_operations_of_a_sixteen_tick_bar_store_twenty_souls() {
        let mut base = LaserBase::from_saved(0, None, Some(16));
        let operations = run_with(&mut base, true, WARDEN, 160, Some(0.8));
        assert_eq!(operations, 10, "160 ticks, sixteen a bar");
        assert_eq!(base.souls, 2 * operations);
        let mut plain = LaserBase::from_saved(0, None, None);
        assert_eq!(
            run(&mut plain, true, WARDEN, 160),
            8,
            "the same window at the plain bar"
        );
    }

    #[test]
    fn a_saved_bar_is_kept_and_a_zero_one_is_not() {
        let mut machine = LaserBase::from_saved(0, None, Some(16));
        assert_eq!(machine.bar, 16);
        assert_eq!(machine.progress, 16, "a fresh machine starts full");
        assert_eq!(LaserBase::from_saved(0, Some(5), Some(16)).progress, 5);
        assert_eq!(LaserBase::from_saved(0, Some(99), Some(16)).progress, 16);
        assert_eq!(
            LaserBase::from_saved(0, None, Some(0)).bar,
            MAX_PROGRESS,
            "a saved zero bar is no bar"
        );
        assert_eq!(run_with(&mut machine, true, WARDEN, 16, Some(0.8)), 1);
    }

    #[test]
    fn a_network_holds_four_souls_a_pipe() {
        let network = Network::new([0, 0, 0]);
        assert_eq!(network.capacity(), 3 * SOUL_PER_PIPE);
        assert_eq!(network.total(), 0);
        assert_eq!(network.pipes(), &[0, 0, 0]);
        assert_eq!(Network::new([]).capacity(), 0, "a lone block is no network");
    }

    #[test]
    fn adding_fills_pipes_in_walk_order() {
        let mut network = Network::new([0, 0]);
        assert_eq!(network.add(6), 6);
        assert_eq!(
            network.pipes(),
            &[4, 2],
            "the first pipe fills before the second"
        );
        assert_eq!(network.add(9), 2, "two more fit");
        assert_eq!(network.total(), 8);
        assert_eq!(network.add(1), 0, "a full network takes nothing");
    }

    #[test]
    fn draining_empties_pipes_in_walk_order() {
        let mut network = Network::new([4, 4]);
        assert_eq!(network.drain(1), 1);
        assert_eq!(network.pipes(), &[3, 4]);
        assert_eq!(network.drain(5), 5);
        assert_eq!(network.pipes(), &[0, 2]);
        assert_eq!(network.drain(9), 2, "only what is there comes out");
        assert_eq!(network.total(), 0);
    }

    #[test]
    fn a_pipe_moves_four_a_tick_from_a_handler() {
        let mut network = Network::new([0]);
        assert_eq!(pipe_pull(10, &mut network), 4, "four a tick, not ten");
        assert_eq!(network.total(), 4);
        assert_eq!(pipe_pull(10, &mut network), 0, "a full pipe takes nothing");
        let mut wide = Network::new([0, 0, 0]);
        assert_eq!(pipe_pull(2, &mut wide), 2, "a handler with two gives two");
        assert_eq!(
            pipe_pull(1, &mut Network::new([])),
            0,
            "no pipe, nowhere to put it"
        );
    }

    #[test]
    fn a_surge_buys_one_soul_for_three_hundred_ticks() {
        let mut network = Network::new([4, 4]);
        let mut surge = Surge::from_saved(0);
        let first = surge.tick(&mut network, Target::Block, 1.0);
        assert_eq!(first.souls_spent, 1, "the first tick buys a soul");
        assert_eq!(
            surge.ticking_time,
            SOUL_TIME - 1,
            "and spends one of its ticks"
        );
        assert_eq!(network.total(), 7);
        let mut spent = 1;
        for _ in 0..SOUL_TIME - 1 {
            spent += surge.tick(&mut network, Target::Block, 1.0).souls_spent;
        }
        assert_eq!(surge.ticking_time, 0, "300 ticks use the soul up");
        assert_eq!(spent, 1, "one soul bought the whole 300");
        assert_eq!(network.total(), 7);
        // And the next tick buys a second.
        assert_eq!(surge.tick(&mut network, Target::Block, 1.0).souls_spent, 1);
        assert_eq!(network.total(), 6);
    }

    #[test]
    fn a_surge_with_no_souls_does_nothing() {
        let mut network = Network::new([0, 0]);
        let mut surge = Surge::from_saved(0);
        for _ in 0..50 {
            let step = surge.tick(&mut network, Target::Block, 0.0);
            assert_eq!(step, SurgeStep::default(), "no soul, no acceleration");
        }
        assert_eq!(surge.ticking_time, 0);
    }

    #[test]
    fn the_three_percent_roll_gives_four_random_ticks() {
        let mut network = Network::new([4]);
        let mut surge = Surge::from_saved(SOUL_TIME as i32);
        let hit = surge.tick(&mut network, Target::Block, 0.02);
        assert_eq!(
            hit.random_ticks, BLOCK_ACCELERATION_TICK,
            "under 3% is a hit"
        );
        assert_eq!(
            surge.ticking_time,
            SOUL_TIME - 1,
            "a miss still burns a tick"
        );
        let mut surge = Surge::from_saved(SOUL_TIME as i32);
        let miss = surge.tick(&mut network, Target::Block, 0.5);
        assert_eq!(miss.random_ticks, 0, "5% is a miss");
        assert_eq!(surge.ticking_time, SOUL_TIME - 1);
        let boundary = Surge::from_saved(SOUL_TIME as i32).tick(
            &mut network,
            Target::Block,
            RANDOM_TICK_ACCELERATION_CHANCE,
        );
        assert_eq!(boundary.random_ticks, 0, "exactly 3% is a miss");
    }

    #[test]
    fn a_surge_facing_nothing_keeps_its_soul() {
        let mut network = Network::new([4]);
        let mut surge = Surge::from_saved(0);
        assert_eq!(
            surge.tick(&mut network, Target::Nothing, 0.0).souls_spent,
            1
        );
        for _ in 0..(SOUL_TIME * 2) {
            let step = surge.tick(&mut network, Target::Nothing, 0.0);
            assert_eq!(step.random_ticks, 0);
            assert_eq!(step.souls_spent, 0, "a soul is not burned on nothing");
        }
        assert_eq!(
            surge.ticking_time, SOUL_TIME,
            "the clock waits for a target"
        );
        assert_eq!(network.total(), 3);
    }

    #[test]
    fn a_negative_saved_counter_is_an_empty_one() {
        assert_eq!(Surge::from_saved(-20).ticking_time, 0);
        assert_eq!(Surge::from_saved(7).ticking_time, 7);
    }

    // -----------------------------------------------------------------------
    // The review fixes: a swapped lens, and a network walked once a tick
    // -----------------------------------------------------------------------

    /// A second lens is a swap, not a void: the old one goes into the hand that empties, or the
    /// first free hotbar/main/off-hand slot; a full inventory refuses the click.
    #[test]
    fn a_replaced_lens_goes_back_into_the_hand_or_the_first_free_slot() {
        assert_eq!(
            replacement_slot(&[0, 1, 2], 0, 1),
            Some(0),
            "a single held lens leaves its slot empty for the old one"
        );
        assert_eq!(
            replacement_slot(&[0, 1, 2], 0, 2),
            Some(3),
            "a spare in the hand keeps it, so the old lens takes the first free slot"
        );
        assert_eq!(
            replacement_slot(&[0, 1, 40], 0, 3),
            Some(2),
            "the off hand is a destination only after the main inventory"
        );
        let full: Vec<u32> = (0..=35).chain([40]).collect();
        assert_eq!(
            replacement_slot(&full, 0, 2),
            None,
            "a full inventory refuses the click rather than voiding the old lens"
        );
    }

    /// A 200-pipe line is walked once: each position appears once, and the level is asked once per
    /// neighbour, not once per pipe times the line.
    #[test]
    fn a_two_hundred_pipe_line_is_walked_once_a_tick() {
        let mut probes = 0;
        let positions = grow_component([[0, 0, 0]], MAX_NETWORK_PIPES, &mut |pos| {
            probes += 1;
            pos[1] == 0 && pos[2] == 0 && (0..200).contains(&pos[0])
        });
        assert_eq!(
            positions.len(),
            200,
            "every pipe of the line is in the walk"
        );
        assert_eq!(
            positions.iter().filter(|pos| **pos == [42, 0, 0]).count(),
            1,
            "no pipe is visited twice"
        );
        assert!(
            probes <= 6 * 200,
            "one walk probes each pipe's six sides once: {probes} probes"
        );

        // The rest of the tick reuses the walk: the other 199 pipes ask the level nothing.
        let first = probes;
        let mut cache = NetworkCache::new();
        cache.begin(9);
        cache.insert(Walked::new(positions, std::iter::repeat_n(0, 200)));
        for x in 0..200 {
            assert!(
                cache.component([x, 0, 0]).is_some(),
                "pipe {x} is part of the cached component"
            );
        }
        assert_eq!(
            probes, first,
            "a cached component is not walked again this tick"
        );
        assert!(
            cache.component([200, 0, 0]).is_none(),
            "a position outside the component is not answered"
        );
        cache.begin(10);
        assert!(
            cache.component([0, 0, 0]).is_none(),
            "the next tick drops the walk"
        );
    }

    /// The work budget: a component larger than [`MAX_NETWORK_PIPES`] stops at the bound.
    #[test]
    fn a_network_walk_stops_at_the_budget() {
        let mut probes = 0;
        let positions = grow_component([[0, 0, 0]], 100, &mut |pos| {
            probes += 1;
            pos[1] == 0 && pos[2] == 0 && (0..5_000).contains(&pos[0])
        });
        assert_eq!(positions.len(), 100, "the walk is cut at the budget");
        assert!(
            probes <= 6 * 100,
            "the cut also bounds the probes: {probes}"
        );
        assert_eq!(positions[0], [0, 0, 0]);
    }

    // -----------------------------------------------------------------------
    // Waterlogging (`SimpleWaterloggedBlock`)
    // -----------------------------------------------------------------------

    /// A full bucket fills a dry machine and is refused by a waterlogged one:
    /// `SimpleWaterloggedBlock.canPlaceLiquid` is `!state.getValue(WATERLOGGED)`.
    #[test]
    fn a_water_bucket_fills_a_dry_block_and_refuses_a_full_one() {
        assert_eq!(
            water_click(WATER_BUCKET, Waterlogged::Dry),
            Some(WaterClick::Fill),
            "a full bucket empties into a dry block"
        );
        assert_eq!(
            water_click(WATER_BUCKET, Waterlogged::Holding),
            None,
            "a block already waterlogged takes nothing"
        );
        assert!(!Waterlogged::Dry.holds_water());
    }

    /// An empty bucket drains a waterlogged machine and gets nothing from a dry one
    /// (`BucketPickup.pickupBlock`).
    #[test]
    fn an_empty_bucket_drains_a_waterlogged_block_and_refuses_a_dry_one() {
        assert_eq!(
            water_click(BUCKET, Waterlogged::Holding),
            Some(WaterClick::Drain),
            "an empty bucket takes the water out"
        );
        assert_eq!(
            water_click(BUCKET, Waterlogged::Dry),
            None,
            "a dry block gives no water"
        );
        assert!(Waterlogged::Holding.holds_water());
    }

    /// Nothing but a bucket is a water click: the lens and the augment clicks pass through, and an
    /// empty hand is `use_without_item`'s, not this rule's.
    #[test]
    fn anything_but_a_bucket_is_not_a_water_click() {
        for item in [
            LENS,
            "minecraft:air",
            "minecraft:water_bottle",
            "industrialforegoing:efficiency_addon_tier_2",
        ] {
            assert_eq!(water_click(item, Waterlogged::Dry), None, "{item}");
            assert_eq!(water_click(item, Waterlogged::Holding), None, "{item}");
        }
    }

    /// The drain's full bucket goes to the first free slot that is not the clicking hand, and never
    /// to a slot that already holds something; a full inventory refuses the drain.
    #[test]
    fn a_drained_bucket_lands_in_the_first_free_slot_but_not_the_hand() {
        assert_eq!(
            bucket_destination(&[0], 0),
            Some(1),
            "the hand is not a destination: the emptied slot is where vanilla would put it"
        );
        assert_eq!(
            bucket_destination(&[0, 1, 2], 0),
            Some(3),
            "the first free slot after the hand"
        );
        assert_eq!(
            bucket_destination(&[0, 1, 41], 2),
            Some(3),
            "a free hand slot is skipped for the next free one"
        );
        let full: Vec<u32> = (0..=35).collect();
        assert_eq!(
            bucket_destination(&full, 0),
            Some(40),
            "the off hand is a destination after the main inventory"
        );
        let everything: Vec<u32> = (0..=35).chain([40]).collect();
        assert_eq!(
            bucket_destination(&everything, 0),
            None,
            "a full inventory refuses the drain rather than losing the water"
        );
    }

    /// The state's own value: only `true` is waterlogged, and the two states spell themselves the
    /// way the property is written.
    #[test]
    fn only_the_property_true_is_waterlogged() {
        assert_eq!(
            Waterlogged::from_property(Some("true")),
            Waterlogged::Holding
        );
        assert_eq!(Waterlogged::from_property(Some("false")), Waterlogged::Dry);
        assert_eq!(Waterlogged::from_property(None), Waterlogged::Dry);
        assert_eq!(Waterlogged::from_property(Some("")), Waterlogged::Dry);
        assert_eq!(Waterlogged::Holding.property(), "true");
        assert_eq!(Waterlogged::Dry.property(), "false");
        assert_eq!(Waterlogged::default(), Waterlogged::Dry);
    }

    /// The cached component keeps this tick's transfer, including the ticking pipe's own value.
    #[test]
    fn a_cached_component_keeps_the_ticks_transfer() {
        let mut cache = NetworkCache::new();
        cache.begin(3);
        cache.insert(Walked::new(vec![[0, 0, 0], [1, 0, 0]], [1, 2]));
        cache.set_souls([0, 0, 0], 4);
        let walked = cache.component([1, 0, 0]).expect("the line is cached");
        assert_eq!(
            walked.network().pipes(),
            &[4, 2],
            "the write-back is visible"
        );
        assert_eq!(walked.network().total(), 6);
    }

    /// The menu's three data slots are the tank, the bar's progress and the bar's length, in that
    /// order: what `SoulLaserBaseScreen`'s `ContainerData` and this port's screen draw.
    #[test]
    fn the_menu_syncs_the_tank_the_progress_and_the_bar() {
        let base = LaserBase::from_saved(12, Some(5), Some(16));
        assert_eq!(menu_data(&base), [12, 5, 16]);
    }

    /// The data slots travel as 16-bit values (vanilla's `ContainerData` is a short), so nothing the
    /// menu sends may exceed the wire's top: a value is clamped, never wrapped. (A `LaserBase` read
    /// from a save is already clamped to the tank; the rule is tested on a value that is not.)
    #[test]
    fn a_data_slot_stops_at_the_sixteen_bit_top() {
        let base = LaserBase {
            souls: 40_000,
            progress: 5,
            bar: 16,
        };
        assert_eq!(menu_data(&base), [i32::from(i16::MAX), 5, 16]);
    }

    /// An efficiency addon clicks into the first empty augment slot (1), then the second (2).
    #[test]
    fn an_efficiency_addon_goes_into_the_first_empty_augment_slot() {
        assert_eq!(
            augment_slot([true, true], Addon::Efficiency, false),
            Some(1)
        );
        assert_eq!(
            augment_slot([false, true], Addon::Efficiency, true),
            None,
            "one is already seated"
        );
    }

    /// A second efficiency addon is refused, in the occupied slot and in the empty one: the machine
    /// holds one. That is `canAcceptAugment`'s "a type already seated" rule, and the click that the
    /// e2e drives into an occupied augment slot depends on it.
    #[test]
    fn a_second_efficiency_addon_is_refused_by_either_augment_slot() {
        assert!(!augment_fits(Addon::Efficiency, true));
        for slot in AUGMENT_SLOTS {
            assert!(
                !slot_accepts(slot, false, Addon::Efficiency, true),
                "augment slot {slot} took a second efficiency addon"
            );
        }
    }

    /// A speed addon is never an augment this machine takes (`canAcceptAugment`), seated or not.
    #[test]
    fn a_speed_addon_is_refused() {
        assert!(!augment_fits(Addon::Speed, false));
        assert!(!augment_fits(Addon::Speed, true));
        assert!(!slot_accepts(AUGMENT_SLOTS[0], false, Addon::Speed, false));
    }

    /// The catalyst slot takes the blue lens and nothing else — including nothing but the lens'
    /// item id, so an addon held over it is refused.
    #[test]
    fn the_catalyst_slot_takes_only_the_blue_lens() {
        assert!(slot_accepts(LENS_SLOT, true, Addon::None, false));
        assert!(!slot_accepts(LENS_SLOT, false, Addon::Efficiency, false));
    }

    /// The player's inventory (menu slots past the container) is not this machine's to refuse.
    #[test]
    fn the_player_inventory_slots_are_not_the_machines_to_refuse() {
        assert!(slot_accepts(SLOT_COUNT, false, Addon::Speed, true));
        assert!(slot_accepts(SLOT_COUNT + 36, false, Addon::None, true));
    }

    /// A save from 0.2.x holds the lens and the augment's item id under the old keys and no slots:
    /// the first tick adopts them into the container, positionally, and leaves the rest empty.
    #[test]
    fn a_legacy_save_adopts_its_lens_and_augments_into_the_slots() {
        assert_eq!(
            adopted_items(
                Some(LENS),
                [Some("industrialforegoing:efficiency_addon_tier_2"), None]
            ),
            [
                Some(LENS.to_owned()),
                Some("industrialforegoing:efficiency_addon_tier_2".to_owned()),
                None
            ]
        );
        assert_eq!(adopted_items(None, [None, None]), [None, None, None]);
    }
}
