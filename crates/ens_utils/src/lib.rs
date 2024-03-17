//! General utilities for [Ens].
//!
//! [Ens]: https://github.com/AxiomaticSemantics/ens
//!

#[allow(missing_docs)]
pub mod prelude {
    pub use crate::default;
}

#[cfg(feature = "futures")]
pub mod futures;
pub mod hash;
pub mod label;
#[cfg(feature = "short_names")]
mod short_names;
#[cfg(feature = "short_names")]
pub use short_names::get_short_name;

mod once;
pub mod synccell;
pub mod syncunsafecell;

#[cfg(feature = "cow_arc")]
mod cow_arc;
mod default;
pub mod intern;
#[cfg(feature = "parallel")]
mod parallel_queue;

//pub use ahash::{AHasher, RandomState};
#[cfg(feature = "cow_arc")]
pub use cow_arc::*;
pub use default::default;
pub use ens_utils_proc_macros::*;
pub use hash::*;
#[cfg(feature = "parallel")]
pub use parallel_queue::*;

use std::{future::Future, mem::ManuallyDrop, pin::Pin};

/// An owned and dynamically typed Future used when you can't statically type your result or need to add some indirection.
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A type which calls a function when dropped.
/// This can be used to ensure that cleanup code is run even in case of a panic.
///
/// Note that this only works for panics that [unwind](https://doc.rust-lang.org/nomicon/unwinding.html)
/// -- any code within `OnDrop` will be skipped if a panic does not unwind.
/// In most cases, this will just work.
///
/// # Examples
///
/// ```
/// # use ens_utils::OnDrop;
/// # fn test_panic(do_panic: bool, log: impl FnOnce(&str)) {
/// // This will print a message when the variable `_catch` gets dropped,
/// // even if a panic occurs before we reach the end of this scope.
/// // This is similar to a `try ... catch` block in languages such as C++.
/// let _catch = OnDrop::new(|| log("Oops, a panic occurred and this function didn't complete!"));
///
/// // Some code that may panic...
/// // ...
/// # if do_panic { panic!() }
///
/// // Make sure the message only gets printed if a panic occurs.
/// // If we remove this line, then the message will be printed regardless of whether a panic occurs
/// // -- similar to a `try ... finally` block.
/// std::mem::forget(_catch);
/// # }
/// #
/// # test_panic(false, |_| unreachable!());
/// # let mut did_log = false;
/// # std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
/// #   test_panic(true, |_| did_log = true);
/// # }));
/// # assert!(did_log);
/// ```
pub struct OnDrop<F: FnOnce()> {
    callback: ManuallyDrop<F>,
}

impl<F: FnOnce()> OnDrop<F> {
    /// Returns an object that will invoke the specified callback when dropped.
    pub fn new(callback: F) -> Self {
        Self {
            callback: ManuallyDrop::new(callback),
        }
    }
}

impl<F: FnOnce()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        // SAFETY: We may move out of `self`, since this instance can never be observed after it's dropped.
        let callback = unsafe { ManuallyDrop::take(&mut self.callback) };
        callback();
    }
}
