#![doc = include_str!("../README.md")]

/// Common run conditions
#[cfg(feature = "common_conditions")]
pub mod common_conditions;
mod real;
#[allow(clippy::module_inception)]
mod time;

#[cfg(feature = "timers")]
mod stopwatch;
#[cfg(feature = "timers")]
mod timer;

pub use real::*;
pub use time::*;

#[cfg(feature = "timers")]
pub use stopwatch::*;
#[cfg(feature = "timers")]
pub use timer::*;

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{Real, Time};

    #[cfg(feature = "timers")]
    pub use crate::{Stopwatch, Timer, TimerMode};
}

#[cfg(feature = "events")]
use ens::event::{signal_event_update_system, EventUpdateSignal, EventUpdates};
use ens::prelude::*;
use ens_app::{prelude::*, PreUpdate};
use std::time::{Duration, Instant};

/// Adds time functionality to Apps.
#[derive(Default)]
pub struct TimePlugin;

#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
/// Updates the elapsed time. Any system that interacts with [`Time`] component should run after
/// this.
pub struct TimeSystem;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time<Real>>()
            .init_resource::<TimeUpdateStrategy>()
            .add_systems(PreUpdate, time_system.in_set(TimeSystem));
    }
}

/// Configuration resource used to determine how the time system should run.
///
/// For most cases, [`TimeUpdateStrategy::Automatic`] is fine. When writing tests, dealing with
/// networking or similar, you may prefer to set the next [`Time`] value manually.
#[derive(Resource, Default)]
pub enum TimeUpdateStrategy {
    /// [`Time`] will be automatically updated each frame using an [`Instant`] sent from the render world via a [`TimeSender`].
    /// If nothing is sent, the system clock will be used instead.
    #[default]
    Automatic,
    /// [`Time`] will be updated to the specified [`Instant`] value each frame.
    /// In order for time to progress, this value must be manually updated each frame.
    ///
    /// Note that the `Time` resource will not be updated until [`TimeSystem`] runs.
    ManualInstant(Instant),
    /// [`Time`] will be incremented by the specified [`Duration`] each frame.
    ManualDuration(Duration),
}

/// The system used to update the [`Time`] used by app logic.
fn time_system(mut time: ResMut<Time<Real>>, update_strategy: Res<TimeUpdateStrategy>) {
    match update_strategy.as_ref() {
        TimeUpdateStrategy::Automatic => time.update_with_instant(Instant::now()),
        TimeUpdateStrategy::ManualInstant(instant) => time.update_with_instant(*instant),
        TimeUpdateStrategy::ManualDuration(duration) => time.update_with_duration(*duration),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Real, Time, TimePlugin, TimeUpdateStrategy};
    use ens::event::{Event, EventReader, EventWriter};
    use ens_app::{App, Startup, Update};
    use std::error::Error;
    use std::time::Duration;

    #[derive(Event)]
    struct TestEvent<T: Default> {
        sender: std::sync::mpsc::Sender<T>,
    }

    impl<T: Default> Drop for TestEvent<T> {
        fn drop(&mut self) {
            self.sender
                .send(T::default())
                .expect("Failed to send drop signal");
        }
    }

    #[test]
    fn events_get_dropped_regression_test_11528() -> Result<(), impl Error> {
        let (tx1, rx1) = std::sync::mpsc::channel();
        let (tx2, rx2) = std::sync::mpsc::channel();
        let mut app = App::new();
        app.add_plugins(TimePlugin)
            .add_event::<TestEvent<i32>>()
            .add_event::<TestEvent<()>>()
            .add_systems(Startup, move |mut ev2: EventWriter<TestEvent<()>>| {
                ev2.send(TestEvent {
                    sender: tx2.clone(),
                });
            })
            .add_systems(Update, move |mut ev1: EventWriter<TestEvent<i32>>| {
                // Keep adding events so this event type is processed every update
                ev1.send(TestEvent {
                    sender: tx1.clone(),
                });
            })
            .add_systems(
                Update,
                |mut ev1: EventReader<TestEvent<i32>>, mut ev2: EventReader<TestEvent<()>>| {
                    // Read events so they can be dropped
                    for _ in ev1.read() {}
                    for _ in ev2.read() {}
                },
            )
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                1. / 60.,
            )));

        for _ in 0..10 {
            app.update();
        }

        // Check event type 1 as been dropped at least once
        let _drop_signal = rx1.try_recv()?;
        // Check event type 2 has been dropped
        rx2.try_recv()
    }
}
