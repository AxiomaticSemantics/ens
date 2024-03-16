use crate::{App, Plugin, PostUpdate, PreUpdate, Update};
use ens::system::NonSend;
use ens_tasks::{tick_global_task_pools_on_main_thread, TaskPoolOptions};

use std::marker::PhantomData;

/// Setup of default task pools: [`AsyncComputeTaskPool`](ens_tasks::AsyncComputeTaskPool),
/// [`ComputeTaskPool`](ens_tasks::ComputeTaskPool), [`IoTaskPool`](ens_tasks::IoTaskPool).
#[derive(Default)]
pub struct TaskPoolPlugin {
    /// Options for the [`TaskPool`](ens_tasks::TaskPool) created at application start.
    pub task_pool_options: TaskPoolOptions,
}

impl Plugin for TaskPoolPlugin {
    fn build(&self, app: &mut App) {
        // Setup the default ens task pools
        self.task_pool_options.create_default_pools();

        app.add_systems(PostUpdate, tick_global_task_pools);
        app.add_systems(Update, tick_global_task_pools);
        app.add_systems(PreUpdate, tick_global_task_pools);
    }
}
/// A dummy type that is [`!Send`](Send), to force systems to run on the main thread.
pub struct NonSendMarker(PhantomData<*mut ()>);

/// A system used to check and advanced our task pools.
///
/// Calls [`tick_global_task_pools_on_main_thread`],
/// and uses [`NonSendMarker`] to ensure that this system runs on the main thread
#[inline(always)]
fn tick_global_task_pools(_main_thread_marker: Option<NonSend<NonSendMarker>>) {
    tick_global_task_pools_on_main_thread();
}
