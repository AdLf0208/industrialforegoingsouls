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
}
