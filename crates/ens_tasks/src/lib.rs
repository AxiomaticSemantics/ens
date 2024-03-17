#![doc = include_str!("../README.md")]

#[cfg(feature = "multi-threaded")]
mod slice;
#[cfg(feature = "multi-threaded")]
pub use slice::{ParallelSlice, ParallelSliceMut};

mod task;
pub use task::Task;

#[cfg(feature = "multi-threaded")]
mod task_pool_options;
#[cfg(feature = "multi-threaded")]
pub use task_pool_options::*;

#[cfg(feature = "multi-threaded")]
mod task_pool;
#[cfg(feature = "multi-threaded")]
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};

#[cfg(not(feature = "multi-threaded"))]
mod single_threaded_task_pool;
#[cfg(not(feature = "multi-threaded"))]
pub use single_threaded_task_pool::{FakeTask, Scope, TaskPool, TaskPoolBuilder, ThreadExecutor};

#[cfg(feature = "multi-threaded")]
mod usages;
#[cfg(feature = "multi-threaded")]
pub use usages::tick_global_task_pools_on_main_thread;

#[cfg(feature = "async_compute_task_pool")]
pub use usages::AsyncComputeTaskPool;

#[cfg(feature = "compute_task_pool")]
pub use usages::ComputeTaskPool;

#[cfg(feature = "io_task_pool")]
pub use usages::IoTaskPool;

#[cfg(feature = "multi-threaded")]
mod thread_executor;
#[cfg(feature = "multi-threaded")]
pub use thread_executor::{ThreadExecutor, ThreadExecutorTicker};

#[cfg(feature = "async-io")]
pub use async_io::block_on;
#[cfg(not(feature = "async-io"))]
pub use futures_lite::future::block_on;
pub use futures_lite::future::poll_once;

mod iter;
#[cfg(feature = "multi-threaded")]
pub use iter::ParallelIterator;

pub use futures_lite;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        block_on,
        task_pool::TaskPoolBuilder,
        //iter::ParallelIterator,
        //slice::{ParallelSlice, ParallelSliceMut},
        //usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool},
    };
}

use std::num::NonZeroUsize;

/// Gets the logical CPU core count available to the current process.
///
/// This is identical to [`std::thread::available_parallelism`], except
/// it will return a default value of 1 if it internally errors out.
///
/// This will always return at least 1.
#[cfg(feature = "multi-threaded")]
pub fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1)
}
