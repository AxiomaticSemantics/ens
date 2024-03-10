use crate::{
    app::{App, AppExit},
    plugin::Plugin,
    PluginsState,
};
use ens::event::{Events, ManualEventReader};
#[cfg(feature = "loop_wait")]
use std::time::{Duration, Instant};

/// Determines the method used to run an [`App`]'s [`Schedule`](ens::schedule::Schedule).
///
/// It is used in the [`ScheduleRunnerPlugin`].
#[derive(Copy, Clone, Debug)]
pub enum RunMode {
    /// Indicates that the [`App`]'s schedule should run repeatedly.
    Loop,
    #[cfg(feature = "loop_wait")]
    LoopWait {
        /// The minimum [`Duration`] to wait after a [`Schedule`](ens::schedule::Schedule)
        /// has completed before repeating. A value of [`None`] will not wait.
        wait: Duration,
    },
    /// Indicates that the [`App`]'s schedule should run only once.
    Once,
}

impl Default for RunMode {
    fn default() -> Self {
        RunMode::Loop
    }
}

/// Configures an [`App`] to run its [`Schedule`](ens::schedule::Schedule) according to a given
/// [`RunMode`].
///
/// [`ScheduleRunnerPlugin`] is included in the
/// [`MinimalPlugins`](https://docs.rs/bevy/latest/bevy/struct.MinimalPlugins.html) plugin group.
///
/// [`ScheduleRunnerPlugin`] is *not* included in the
/// [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html) plugin group
/// which assumes that the [`Schedule`](ens::schedule::Schedule) will be executed by other means
/// such as an external event loop that handles execution of the schedule hence making
/// [`ScheduleRunnerPlugin`] unnecessary.
#[derive(Default)]
pub struct ScheduleRunnerPlugin {
    /// Determines whether the [`Schedule`](ens::schedule::Schedule) is run once or repeatedly.
    pub run_mode: RunMode,
}

impl ScheduleRunnerPlugin {
    /// See [`RunMode::Once`].
    pub fn run_once() -> Self {
        ScheduleRunnerPlugin {
            run_mode: RunMode::Once,
        }
    }

    /// See [`RunMode::Loop`].
    pub fn run_loop() -> Self {
        ScheduleRunnerPlugin {
            run_mode: RunMode::Loop,
        }
    }

    /// See [`RunMode::Loop`].
    #[cfg(feature = "loop_wait")]
    pub fn run_loop_wait(wait_duration: Duration) -> Self {
        ScheduleRunnerPlugin {
            run_mode: RunMode::LoopWait {
                wait: wait_duration,
            },
        }
    }
}

impl Plugin for ScheduleRunnerPlugin {
    fn build(&self, app: &mut App) {
        let run_mode = self.run_mode;
        app.set_runner(move |mut app: App| {
            let plugins_state = app.plugins_state();
            if plugins_state != PluginsState::Cleaned {
                while app.plugins_state() == PluginsState::Adding {
                    ens_tasks::tick_global_task_pools_on_main_thread();
                }
                app.finish();
                app.cleanup();
            }

            let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
            match run_mode {
                RunMode::Once => app.update(),
                RunMode::Loop => loop {
                    app.update();
                    if let Some(app_exit_events) = app.world.get_resource_mut::<Events<AppExit>>() {
                        if let Some(exit) = app_exit_event_reader.read(&app_exit_events).last() {
                            break;
                        }
                    }
                },
                #[cfg(feature = "loop_wait")]
                RunMode::LoopWait { wait } => {
                    let mut tick = move |app: &mut App,
                                         wait: Duration|
                          -> Result<Option<Duration>, AppExit> {
                        let start_time = Instant::now();

                        app.update();

                        if let Some(app_exit_events) =
                            app.world.get_resource_mut::<Events<AppExit>>()
                        {
                            if let Some(exit) = app_exit_event_reader.read(&app_exit_events).last()
                            {
                                return Err(exit.clone());
                            }
                        }

                        let end_time = Instant::now();
                        let exe_time = end_time - start_time;
                        if exe_time < wait {
                            return Ok(Some(wait - exe_time));
                        }

                        Ok(None)
                    };

                    while let Ok(delay) = tick(&mut app, wait) {
                        if let Some(delay) = delay {
                            std::thread::sleep(delay);
                        }
                    }
                }
            }
        });
    }
}
