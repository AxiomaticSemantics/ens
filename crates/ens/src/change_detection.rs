//! Types that detect when their internal data mutate.

use crate::{
    access::MutUntyped,
    component::{Tick, TickCells},
    ptr::PtrMut,
    system::Resource,
};

use ens_ptr::{Ptr, UnsafeCellDeref};

use std::mem;
use std::ops::{Deref, DerefMut};

/// The (arbitrarily chosen) minimum number of world tick increments between `check_tick` scans.
///
/// Change ticks can only be scanned when systems aren't running. Thus, if the threshold is `N`,
/// the maximum is `2 * N - 1` (i.e. the world ticks `N - 1` times, then `N` times).
///
/// If no change is older than `u32::MAX - (2 * N - 1)` following a scan, none of their ages can
/// overflow and cause false positives.
// (518,400,000 = 1000 ticks per frame * 144 frames per second * 3600 seconds per hour)
pub const CHECK_TICK_THRESHOLD: u32 = 518_400_000;

/// The maximum change tick difference that won't overflow before the next `check_tick` scan.
///
/// Changes stop being detected once they become this old.
pub const MAX_CHANGE_AGE: u32 = u32::MAX - (2 * CHECK_TICK_THRESHOLD - 1);

/// Types that can read change detection information.
/// This change detection is controlled by [`DetectChangesMut`] types such as [`ResMut`].
///
/// ## Example
/// Using types that implement [`DetectChanges`], such as [`Res`], provide
/// a way to query if a value has been mutated in another system.
///
/// ```
/// use ens::prelude::*;
///
/// #[derive(Resource)]
/// struct MyResource(u32);
///
/// fn my_system(mut resource: Res<MyResource>) {
///     if resource.is_changed() {
///         println!("My component was mutated!");
///     }
/// }
/// ```
pub trait DetectChanges {
    /// Returns `true` if this value was added after the system last ran.
    fn is_added(&self) -> bool;

    /// Returns `true` if this value was added or mutably dereferenced
    /// either since the last time the system ran or, if the system never ran,
    /// since the beginning of the program.
    ///
    /// To check if the value was mutably dereferenced only,
    /// use `this.is_changed() && !this.is_added()`.
    fn is_changed(&self) -> bool;

    /// Returns the change tick recording the time this data was most recently changed.
    ///
    /// Note that components and resources are also marked as changed upon insertion.
    ///
    /// For comparison, the previous change tick of a system can be read using the
    /// [`SystemChangeTick`](crate::system::SystemChangeTick)
    /// [`SystemParam`](crate::system::SystemParam).
    fn last_changed(&self) -> Tick;
}

/// Types that implement reliable change detection.
///
/// ## Example
/// Using types that implement [`DetectChangesMut`], such as [`ResMut`], provide
/// a way to query if a value has been mutated in another system.
/// Normally change detection is triggered by either [`DerefMut`] or [`AsMut`], however
/// it can be manually triggered via [`set_changed`](DetectChangesMut::set_changed).
///
/// To ensure that changes are only triggered when the value actually differs,
/// check if the value would change before assignment, such as by checking that `new != old`.
/// You must be *sure* that you are not mutably dereferencing in this process.
///
/// [`set_if_neq`](DetectChangesMut::set_if_neq) is a helper
/// method for this common functionality.
///
/// ```
/// use ens::prelude::*;
///
/// #[derive(Resource)]
/// struct MyResource(u32);
///
/// fn my_system(mut resource: ResMut<MyResource>) {
///     if resource.is_changed() {
///         println!("My resource was mutated!");
///     }
///
///    resource.0 = 42; // triggers change detection via [`DerefMut`]
/// }
/// ```
///
pub trait DetectChangesMut: DetectChanges {
    /// The type contained within this smart pointer
    ///
    /// For example, for `ResMut<T>` this would be `T`.
    type Inner: ?Sized;

    /// Flags this value as having been changed.
    ///
    /// Mutably accessing this smart pointer will automatically flag this value as having been changed.
    /// However, mutation through interior mutability requires manual reporting.
    ///
    /// **Note**: This operation cannot be undone.
    fn set_changed(&mut self);

    /// Manually sets the change tick recording the time when this data was last mutated.
    ///
    /// # Warning
    /// This is a complex and error-prone operation, primarily intended for use with rollback networking strategies.
    /// If you merely want to flag this data as changed, use [`set_changed`](DetectChangesMut::set_changed) instead.
    /// If you want to avoid triggering change detection, use [`bypass_change_detection`](DetectChangesMut::bypass_change_detection) instead.
    fn set_last_changed(&mut self, last_changed: Tick);

    /// Manually bypasses change detection, allowing you to mutate the underlying value without updating the change tick.
    ///
    /// # Warning
    /// This is a risky operation, that can have unexpected consequences on any system relying on this code.
    /// However, it can be an essential escape hatch when, for example,
    /// you are trying to synchronize representations using change detection and need to avoid infinite recursion.
    fn bypass_change_detection(&mut self) -> &mut Self::Inner;

    /// Overwrites this smart pointer with the given value, if and only if `*self != value`.
    /// Returns `true` if the value was overwritten, and returns `false` if it was not.
    ///
    /// This is useful to ensure change detection is only triggered when the underlying value
    /// changes, instead of every time it is mutably accessed.
    ///
    /// If you need the previous value, use [`replace_if_neq`](DetectChangesMut::replace_if_neq).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ens::{prelude::*, schedule::common_conditions::resource_changed};
    /// #[derive(Resource, PartialEq, Eq)]
    /// pub struct Score(u32);
    ///
    /// fn reset_score(mut score: ResMut<Score>) {
    ///     // Set the score to zero, unless it is already zero.
    ///     score.set_if_neq(Score(0));
    /// }
    /// # let mut world = World::new();
    /// # world.insert_resource(Score(1));
    /// # let mut score_changed = IntoSystem::into_system(resource_changed::<Score>);
    /// # score_changed.initialize(&mut world);
    /// # score_changed.run((), &mut world);
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems(reset_score);
    /// #
    /// # // first time `reset_score` runs, the score is changed.
    /// # schedule.run(&mut world);
    /// # assert!(score_changed.run((), &mut world));
    /// # // second time `reset_score` runs, the score is not changed.
    /// # schedule.run(&mut world);
    /// # assert!(!score_changed.run((), &mut world));
    /// ```
    #[inline]
    fn set_if_neq(&mut self, value: Self::Inner) -> bool
    where
        Self::Inner: Sized + PartialEq,
    {
        let old = self.bypass_change_detection();
        if *old != value {
            *old = value;
            self.set_changed();
            true
        } else {
            false
        }
    }

    /// Overwrites this smart pointer with the given value, if and only if `*self != value`,
    /// returning the previous value if this occurs.
    ///
    /// This is useful to ensure change detection is only triggered when the underlying value
    /// changes, instead of every time it is mutably accessed.
    ///
    /// If you don't need the previous value, use [`set_if_neq`](DetectChangesMut::set_if_neq).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ens::{prelude::*, schedule::common_conditions::{resource_changed, on_event}};
    /// #[derive(Resource, PartialEq, Eq)]
    /// pub struct Score(u32);
    ///
    /// #[derive(Event, PartialEq, Eq)]
    /// pub struct ScoreChanged {
    ///     current: u32,
    ///     previous: u32,
    /// }
    ///
    /// fn reset_score(mut score: ResMut<Score>, mut score_changed: EventWriter<ScoreChanged>) {
    ///     // Set the score to zero, unless it is already zero.
    ///     let new_score = 0;
    ///     if let Some(Score(previous_score)) = score.replace_if_neq(Score(new_score)) {
    ///         // If `score` change, emit a `ScoreChanged` event.
    ///         score_changed.send(ScoreChanged {
    ///             current: new_score,
    ///             previous: previous_score,
    ///         });
    ///     }
    /// }
    /// # let mut world = World::new();
    /// # world.insert_resource(Events::<ScoreChanged>::default());
    /// # world.insert_resource(Score(1));
    /// # let mut score_changed = IntoSystem::into_system(resource_changed::<Score>);
    /// # score_changed.initialize(&mut world);
    /// # score_changed.run((), &mut world);
    /// #
    /// # let mut score_changed_event = IntoSystem::into_system(on_event::<ScoreChanged>());
    /// # score_changed_event.initialize(&mut world);
    /// # score_changed_event.run((), &mut world);
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems(reset_score);
    /// #
    /// # // first time `reset_score` runs, the score is changed.
    /// # schedule.run(&mut world);
    /// # assert!(score_changed.run((), &mut world));
    /// # assert!(score_changed_event.run((), &mut world));
    /// # // second time `reset_score` runs, the score is not changed.
    /// # schedule.run(&mut world);
    /// # assert!(!score_changed.run((), &mut world));
    /// # assert!(!score_changed_event.run((), &mut world));
    /// ```
    #[inline]
    #[must_use = "If you don't need to handle the previous value, use `set_if_neq` instead."]
    fn replace_if_neq(&mut self, value: Self::Inner) -> Option<Self::Inner>
    where
        Self::Inner: Sized + PartialEq,
    {
        let old = self.bypass_change_detection();
        if *old != value {
            let previous = mem::replace(old, value);
            self.set_changed();
            Some(previous)
        } else {
            None
        }
    }
}

macro_rules! change_detection_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> DetectChanges for $name<$($generics),*> {
            #[inline]
            fn is_added(&self) -> bool {
                self.ticks
                    .added
                    .is_newer_than(self.ticks.last_run, self.ticks.this_run)
            }

            #[inline]
            fn is_changed(&self) -> bool {
                self.ticks
                    .changed
                    .is_newer_than(self.ticks.last_run, self.ticks.this_run)
            }

            #[inline]
            fn last_changed(&self) -> Tick {
                *self.ticks.changed
            }
        }
    }
}

pub(crate) use change_detection_impl;

macro_rules! change_detection_mut_impl {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> DetectChangesMut for $name<$($generics),*> {
            type Inner = $target;

            #[inline]
            fn set_changed(&mut self) {
                *self.ticks.changed = self.ticks.this_run;
            }

            #[inline]
            fn set_last_changed(&mut self, last_changed: Tick) {
                *self.ticks.changed = last_changed;
            }

            #[inline]
            fn bypass_change_detection(&mut self) -> &mut Self::Inner {
                self.value
            }
        }
    };
}

pub(crate) use change_detection_mut_impl;

#[derive(Clone)]
pub(crate) struct Ticks<'w> {
    pub(crate) added: &'w Tick,
    pub(crate) changed: &'w Tick,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
}

impl<'w> Ticks<'w> {
    /// # Safety
    /// This should never alias the underlying ticks with a mutable one such as `TicksMut`.
    #[inline]
    pub(crate) unsafe fn from_tick_cells(
        cells: TickCells<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            // SAFETY: Caller ensures there is no mutable access to the cell.
            added: unsafe { cells.added.deref() },
            // SAFETY: Caller ensures there is no mutable access to the cell.
            changed: unsafe { cells.changed.deref() },
            last_run,
            this_run,
        }
    }
}

#[cfg(feature = "change_detection")]
pub(crate) struct TicksMut<'w> {
    pub(crate) added: &'w mut Tick,
    pub(crate) changed: &'w mut Tick,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
}

#[cfg(feature = "change_detection")]
impl<'w> TicksMut<'w> {
    /// # Safety
    /// This should never alias the underlying ticks. All access must be unique.
    #[inline]
    pub(crate) unsafe fn from_tick_cells(
        cells: TickCells<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            // SAFETY: Caller ensures there is no alias to the cell.
            added: unsafe { cells.added.deref_mut() },
            // SAFETY: Caller ensures there is no alias to the cell.
            changed: unsafe { cells.changed.deref_mut() },
            last_run,
            this_run,
        }
    }
}

#[cfg(feature = "change_detection")]
impl<'w> From<TicksMut<'w>> for Ticks<'w> {
    fn from(ticks: TicksMut<'w>) -> Self {
        Ticks {
            added: ticks.added,
            changed: ticks.changed,
            last_run: ticks.last_run,
            this_run: ticks.this_run,
        }
    }
}

#[cfg(feature = "change_detection")]
impl<'w> DetectChanges for MutUntyped<'w> {
    #[inline]
    fn is_added(&self) -> bool {
        self.ticks
            .added
            .is_newer_than(self.ticks.last_run, self.ticks.this_run)
    }

    #[inline(always)]
    fn is_changed(&self) -> bool {
        self.ticks
            .changed
            .is_newer_than(self.ticks.last_run, self.ticks.this_run)
    }

    #[inline(always)]
    fn last_changed(&self) -> Tick {
        *self.ticks.changed
    }
}

#[cfg(feature = "change_detection")]
impl<'w> DetectChangesMut for MutUntyped<'w> {
    type Inner = PtrMut<'w>;

    #[inline(always)]
    fn set_changed(&mut self) {
        *self.ticks.changed = self.ticks.this_run;
    }

    #[inline(always)]
    fn set_last_changed(&mut self, last_changed: Tick) {
        *self.ticks.changed = last_changed;
    }

    #[inline(always)]
    fn bypass_change_detection(&mut self) -> &mut Self::Inner {
        &mut self.value
    }
}

#[cfg(test)]
mod tests {
    use ens_macros::Resource;
    use ens_ptr::PtrMut;
    use std::ops::{Deref, DerefMut};

    use crate::{
        self as ens,
        access::{Mut, NonSendMut, Ref, ResMut},
        change_detection::{TicksMut, CHECK_TICK_THRESHOLD, MAX_CHANGE_AGE},
        component::{Component, ComponentTicks, Tick},
        system::{IntoSystem, Query, System},
        world::World,
    };

    use super::{DetectChanges, DetectChangesMut, MutUntyped};

    #[derive(Component, PartialEq)]
    struct C;

    #[derive(Resource)]
    struct R;

    #[derive(Resource, PartialEq)]
    struct R2(u8);

    impl Deref for R2 {
        type Target = u8;
        fn deref(&self) -> &u8 {
            &self.0
        }
    }

    impl DerefMut for R2 {
        fn deref_mut(&mut self) -> &mut u8 {
            &mut self.0
        }
    }

    #[test]
    fn change_expiration() {
        fn change_detected(query: Query<Ref<C>>) -> bool {
            query.single().is_changed()
        }

        fn change_expired(query: Query<Ref<C>>) -> bool {
            query.single().is_changed()
        }

        let mut world = World::new();

        // component added: 1, changed: 1
        world.spawn(C);

        let mut change_detected_system = IntoSystem::into_system(change_detected);
        let mut change_expired_system = IntoSystem::into_system(change_expired);
        change_detected_system.initialize(&mut world);
        change_expired_system.initialize(&mut world);

        // world: 1, system last ran: 0, component changed: 1
        // The spawn will be detected since it happened after the system "last ran".
        assert!(change_detected_system.run((), &mut world));

        // world: 1 + MAX_CHANGE_AGE
        let change_tick = world.change_tick.get_mut();
        *change_tick = change_tick.wrapping_add(MAX_CHANGE_AGE);

        // Both the system and component appeared `MAX_CHANGE_AGE` ticks ago.
        // Since we clamp things to `MAX_CHANGE_AGE` for determinism,
        // `ComponentTicks::is_changed` will now see `MAX_CHANGE_AGE > MAX_CHANGE_AGE`
        // and return `false`.
        assert!(!change_expired_system.run((), &mut world));
    }

    #[test]
    fn change_tick_wraparound() {
        let mut world = World::new();
        world.last_change_tick = Tick::new(u32::MAX);
        *world.change_tick.get_mut() = 0;

        // component added: 0, changed: 0
        world.spawn(C);

        world.increment_change_tick();

        // Since the world is always ahead, as long as changes can't get older than `u32::MAX` (which we ensure),
        // the wrapping difference will always be positive, so wraparound doesn't matter.
        let mut query = world.query::<Ref<C>>();
        assert!(query.single(&world).is_changed());
    }

    #[test]
    fn change_tick_scan() {
        let mut world = World::new();

        // component added: 1, changed: 1
        world.spawn(C);

        // a bunch of stuff happens, the component is now older than `MAX_CHANGE_AGE`
        *world.change_tick.get_mut() += MAX_CHANGE_AGE + CHECK_TICK_THRESHOLD;
        let change_tick = world.change_tick();

        let mut query = world.query::<Ref<C>>();
        for tracker in query.iter(&world) {
            let ticks_since_insert = change_tick.relative_to(*tracker.ticks.added).get();
            let ticks_since_change = change_tick.relative_to(*tracker.ticks.changed).get();
            assert!(ticks_since_insert > MAX_CHANGE_AGE);
            assert!(ticks_since_change > MAX_CHANGE_AGE);
        }

        // scan change ticks and clamp those at risk of overflow
        world.check_change_ticks();

        for tracker in query.iter(&world) {
            let ticks_since_insert = change_tick.relative_to(*tracker.ticks.added).get();
            let ticks_since_change = change_tick.relative_to(*tracker.ticks.changed).get();
            assert_eq!(ticks_since_insert, MAX_CHANGE_AGE);
            assert_eq!(ticks_since_change, MAX_CHANGE_AGE);
        }
    }
}
