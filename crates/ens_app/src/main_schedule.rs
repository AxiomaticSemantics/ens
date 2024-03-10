use crate::{App, Plugin};
use ens::{
    schedule::{ExecutorKind, InternedScheduleLabel, Schedule, ScheduleLabel},
    system::{Local, Resource},
    world::{Mut, World},
};

/// The schedule that contains the app logic that is evaluated each tick of [`App::update()`].
///
/// By default, it will run the following schedules in the given order:
///
/// On the first run of the schedule (and only on the first run), it will run:
/// * [`PreStartup`]
/// * [`Startup`]
/// * [`PostStartup`]
///
/// Then it will run:
/// * [`First`]
/// * [`PreUpdate`]
/// * [`StateTransition`]
/// * [`RunFixedMainLoop`]
///     * This will run [`FixedMain`] zero to many times, based on how much time has elapsed.
/// * [`Update`]
/// * [`PostUpdate`]
/// * [`Last`]
///
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Main;

/// The schedule that runs before [`Startup`].
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreStartup;

/// The schedule that runs once when the app starts.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Startup;

/// The schedule that runs once after [`Startup`].
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostStartup;

/// Runs first in the schedule.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct First;

/// The schedule that contains logic that must run before [`Update`]. For example, a system that reads raw keyboard
/// input OS events into an `Events` resource. This enables systems in [`Update`] to consume the events from the `Events`
/// resource without actually knowing about (or taking a direct scheduler dependency on) the "os-level keyboard event system".
///
/// [`PreUpdate`] exists to do "engine/plugin preparation work" that ensures the APIs consumed in [`Update`] are "ready".
/// [`PreUpdate`] abstracts out "pre work implementation details".
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreUpdate;

/// Runs [state transitions](ens::schedule::States).
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateTransition;

/// The schedule that contains app logic to be ran once per frame.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Update;

/// The schedule that contains logic that must run after [`Update`]. For example, synchronizing "local transforms" in a hierarchy
/// to "global" absolute transforms. This enables the [`PostUpdate`] transform-sync system to react to "local transform" changes in
/// [`Update`] without the [`Update`] systems needing to know about (or add scheduler dependencies for) the "global transform sync system".
///
/// [`PostUpdate`] exists to do "engine/plugin response work" to things that happened in [`Update`].
/// [`PostUpdate`] abstracts out "implementation details" from users defining systems in [`Update`].
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostUpdate;

/// Runs last in the schedule.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Last;

/// Defines the schedules to be run for the [`Main`] schedule, including
/// their order.
#[derive(Resource, Debug)]
pub struct MainScheduleOrder {
    /// The labels to run for the main phase of the [`Main`] schedule (in the order they will be run).
    pub labels: Vec<InternedScheduleLabel>,
    /// The labels to run for the startup phase of the [`Main`] schedule (in the order they will be run).
    pub startup_labels: Vec<InternedScheduleLabel>,
}

impl Default for MainScheduleOrder {
    fn default() -> Self {
        Self {
            labels: vec![
                First.intern(),
                PreUpdate.intern(),
                StateTransition.intern(),
                //RunFixedMainLoop.intern(),
                Update.intern(),
                PostUpdate.intern(),
                Last.intern(),
            ],
            startup_labels: vec![PreStartup.intern(), Startup.intern(), PostStartup.intern()],
        }
    }
}

impl MainScheduleOrder {
    /// Adds the given `schedule` after the `after` schedule in the main list of schedules.
    pub fn insert_after(&mut self, after: impl ScheduleLabel, schedule: impl ScheduleLabel) {
        let index = self
            .labels
            .iter()
            .position(|current| (**current).eq(&after))
            .unwrap_or_else(|| panic!("Expected {after:?} to exist"));
        self.labels.insert(index + 1, schedule.intern());
    }

    /// Adds the given `schedule` after the `after` schedule in the list of startup schedules.
    pub fn insert_startup_after(
        &mut self,
        after: impl ScheduleLabel,
        schedule: impl ScheduleLabel,
    ) {
        let index = self
            .startup_labels
            .iter()
            .position(|current| (**current).eq(&after))
            .unwrap_or_else(|| panic!("Expected {after:?} to exist"));
        self.startup_labels.insert(index + 1, schedule.intern());
    }
}

impl Main {
    /// A system that runs the "main schedule"
    pub fn run_main(world: &mut World, mut run_at_least_once: Local<bool>) {
        if !*run_at_least_once {
            world.resource_scope(|world, order: Mut<MainScheduleOrder>| {
                for &label in &order.startup_labels {
                    let _ = world.try_run_schedule(label);
                }
            });
            *run_at_least_once = true;
        }

        world.resource_scope(|world, order: Mut<MainScheduleOrder>| {
            for &label in &order.labels {
                let _ = world.try_run_schedule(label);
            }
        });
    }
}

/// Initializes the [`Main`] schedule, sub schedules, and resources for a given [`App`].
pub struct MainSchedulePlugin;

impl Plugin for MainSchedulePlugin {
    fn build(&self, app: &mut App) {
        // simple "facilitator" schedules benefit from simpler single threaded scheduling
        let mut main_schedule = Schedule::new(Main);
        main_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        /*
                let mut fixed_main_schedule = Schedule::new(FixedMain);
                fixed_main_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
                let mut fixed_main_loop_schedule = Schedule::new(RunFixedMainLoop);
                fixed_main_loop_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        */
        app.add_schedule(main_schedule)
            //.add_schedule(fixed_main_schedule)
            //.add_schedule(fixed_main_loop_schedule)
            .init_resource::<MainScheduleOrder>()
            //.init_resource::<FixedMainScheduleOrder>()
            .add_systems(Main, Main::run_main);
        //.add_systems(FixedMain, FixedMain::run_fixed_main);
    }
}

/// Defines the schedules to be run for the [`FixedMain`] schedule, including
/// their order.
#[derive(Resource, Debug)]
pub struct FixedMainScheduleOrder {
    /// The labels to run for the [`FixedMain`] schedule (in the order they will be run).
    pub labels: Vec<InternedScheduleLabel>,
}