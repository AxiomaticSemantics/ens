//! This crate provides core functionality for Bevy Engine.

#[cfg(feature = "entity_name")]
mod name;
#[cfg(all(feature = "entity_name", feature = "serialize"))]
mod serde;

use ens::system::Resource;

#[cfg(feature = "entity_name")]
pub use name::*;

pub mod prelude {
    //! The Bevy Core Prelude.
    #[doc(hidden)]
    #[cfg(feature = "frame_count")]
    pub use FrameCountPlugin;

    #[doc(hidden)]
    #[cfg(feature = "entity_name")]
    pub use crate::{DebugName, Name};
}

use ens::prelude::*;
use ens_app::prelude::*;

/// Maintains a count of frames rendered since the start of the application.
///
/// [`FrameCount`] is incremented during [`Last`], providing predictable
/// behavior: it will be 0 during the first update, 1 during the next, and so forth.
///
/// # Overflows
///
/// [`FrameCount`] will wrap to 0 after exceeding [`u32::MAX`]. Within reasonable
/// assumptions, one may exploit wrapping arithmetic to determine the number of frames
/// that have elapsed between two observations – see [`u32::wrapping_sub()`].
#[cfg(feature = "frame_count")]
#[derive(Debug, Default, Resource, Clone, Copy)]
pub struct FrameCount(pub u32);

/// Adds frame counting functionality to Apps.
#[cfg(feature = "frame_count")]
#[derive(Default)]
pub struct FrameCountPlugin;

#[cfg(feature = "frame_count")]
impl Plugin for FrameCountPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCount>();
        app.add_systems(Last, update_frame_count);
    }
}

/// A system used to increment [`FrameCount`] with wrapping addition.
///
/// See [`FrameCount`] for more details.
#[cfg(feature = "frame_count")]
pub fn update_frame_count(mut frame_count: ResMut<FrameCount>) {
    frame_count.0 = frame_count.0.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ens_tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

    #[test]
    fn runs_spawn_local_tasks() {
        let mut app = App::new();
        app.add_plugins(TaskPoolPlugin::default());

        let (async_tx, async_rx) = crossbeam_channel::unbounded();
        AsyncComputeTaskPool::get()
            .spawn_local(async move {
                async_tx.send(()).unwrap();
            })
            .detach();

        let (compute_tx, compute_rx) = crossbeam_channel::unbounded();
        ComputeTaskPool::get()
            .spawn_local(async move {
                compute_tx.send(()).unwrap();
            })
            .detach();

        let (io_tx, io_rx) = crossbeam_channel::unbounded();
        IoTaskPool::get()
            .spawn_local(async move {
                io_tx.send(()).unwrap();
            })
            .detach();

        app.run();

        async_rx.try_recv().unwrap();
        compute_rx.try_recv().unwrap();
        io_rx.try_recv().unwrap();
    }

    #[test]
    fn frame_counter_update() {
        let mut app = App::new();
        app.add_plugins((TaskPoolPlugin::default(), FrameCountPlugin));
        app.update();

        let frame_count = app.world.resource::<FrameCount>();
        assert_eq!(1, frame_count.0);
    }
}
