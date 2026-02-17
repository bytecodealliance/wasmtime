//! Helpers for undoing partial side effects when their larger operation fails.

use core::{fmt, mem, ops};

/// An RAII guard to rollback and undo something on (early) drop.
///
/// Dereferences to its inner `T` and its undo function is given the `T` on
/// drop.
///
/// When all of the changes that need to happen together have happened, you can
/// call `Undo::commit` to disable the guard and commit the associated side
/// effects.
///
/// # Example
///
/// ```
/// use std::cell::Cell;
/// use wasmtime_internal_core::{error::Result, undo::Undo};
///
/// /// Some big ball of state that must always be coherent.
/// pub struct Context {
///     // ...
/// }
///
/// impl Context {
///     /// Perform some incremental mutation to `self`, which might not leave
///     /// it in a valid state unless its whole batch of work is completed.
///     fn do_thing(&mut self, arg: u32) -> Result<()> {
/// #       let _ = arg;
/// #       todo!()
///         // ...
///     }
///
///     /// Undo the side effects of `self.do_thing(arg)` for when we need to
///     /// roll back mutations.
///     fn undo_thing(&mut self, arg: u32) {
/// #       let _ = arg;
///         // ...
///     }
///
///     /// Call `self.do_thing(arg)` for each `arg` in `args`.
///     ///
///     /// However, if any `self.do_thing(arg)` call fails, make sure that
///     /// we roll back to the original state by calling `self.undo_thing(arg)`
///     /// for all the `self.do_thing(arg)` calls that already succeeded. This
///     /// way we never leave `self` in a state where things got half-done.
///     pub fn do_all_or_nothing(&mut self, args: &[u32]) -> Result<()> {
///         // Counter for our progress, so that we know how much to work undo upon
///         // failure.
///         let num_things_done = Cell::new(0);
///
///         // Wrap the `Context` in an `Undo` that rolls back our side effects if
///         // we early-exit this function via `?`-propagation or panic unwinding.
///         let mut ctx = Undo::new(self, |ctx| {
///             for arg in args.iter().take(num_things_done.get()) {
///                 ctx.undo_thing(*arg);
///             }
///         });
///
///         // Do each piece of work!
///         for arg in args {
///             // Note: if this call returns an error that is `?`-propagated or
///             // triggers unwinding by panicking, then the work performed thus
///             // far will be rolled back when `ctx` is dropped.
///             ctx.do_thing(*arg)?;
///
///             // Update how much work has been completed.
///             num_things_done.set(num_things_done.get() + 1);
///         }
///
///         // We completed all of the work, so commit the `Undo` guard and
///         // disable its cleanup function.
///         Undo::commit(ctx);
///
///         Ok(())
///     }
/// }
/// ```
#[must_use = "`Undo` implicitly runs its undo function on drop; use `Undo::commit(...)` \
              to disable"]
pub struct Undo<T, F>
where
    F: FnOnce(T),
{
    inner: mem::ManuallyDrop<T>,
    undo: mem::ManuallyDrop<F>,
}

impl<T, F> Drop for Undo<T, F>
where
    F: FnOnce(T),
{
    fn drop(&mut self) {
        // Safety: These `ManuallyDrop` fields will not be used again.
        let inner = unsafe { mem::ManuallyDrop::take(&mut self.inner) };
        let undo = unsafe { mem::ManuallyDrop::take(&mut self.undo) };
        undo(inner);
    }
}

impl<T, F> fmt::Debug for Undo<T, F>
where
    F: FnOnce(T),
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Undo")
            .field("inner", &self.inner)
            .field("undo", &"..")
            .finish()
    }
}

impl<T, F> ops::Deref for Undo<T, F>
where
    F: FnOnce(T),
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, F> ops::DerefMut for Undo<T, F>
where
    F: FnOnce(T),
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T, F> Undo<T, F>
where
    F: FnOnce(T),
{
    /// Create a new `Undo` guard.
    ///
    /// This guard will wrap the given `inner` object and call `undo(inner)`
    /// when dropped, unless the guard is disabled via `Undo::commit`.
    pub fn new(inner: T, undo: F) -> Self {
        Self {
            inner: mem::ManuallyDrop::new(inner),
            undo: mem::ManuallyDrop::new(undo),
        }
    }

    /// Disable this `Undo` and return its inner value.
    ///
    /// This `Undo`'s cleanup function will never be called.
    pub fn commit(guard: Self) -> T {
        let mut guard = mem::ManuallyDrop::new(guard);

        // Safety: These `ManuallyDrop` fields will not be used again.
        unsafe {
            // Make sure to drop `undo`, even though we aren't calling it, to
            // avoid leaking closed-over `Arc`s, for example.
            mem::ManuallyDrop::drop(&mut guard.undo);

            mem::ManuallyDrop::take(&mut guard.inner)
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::error::{Result, ensure};
    use core::{cell::Cell, cmp};
    use std::{panic, string::ToString};

    #[derive(Default)]
    struct Counter {
        value: u32,
        max_value_seen: u32,
    }

    impl Counter {
        fn inc(&mut self, mut f: impl FnMut(&Self) -> Result<()>) -> Result<()> {
            f(self)?;
            self.value += 1;
            self.max_value_seen = cmp::max(self.max_value_seen, self.value);
            Ok(())
        }

        fn dec(&mut self) {
            self.value -= 1;
        }

        fn inc_n(&mut self, n: u32, mut f: impl FnMut(&Self) -> Result<()>) -> Result<()> {
            let i = Cell::new(0);

            let mut counter = Undo::new(self, |counter| {
                for _ in 0..i.get() {
                    counter.dec();
                }
            });

            for _ in 0..n {
                counter.inc(&mut f)?;
                i.set(i.get() + 1);
            }

            Undo::commit(counter);
            Ok(())
        }
    }

    #[test]
    fn error_propagation() {
        let mut counter = Counter::default();
        let result = counter.inc_n(10, |c| {
            ensure!(c.value < 5, "uh oh");
            Ok(())
        });
        assert_eq!(result.unwrap_err().to_string(), "uh oh");
        assert_eq!(counter.value, 0);
        assert_eq!(counter.max_value_seen, 5);
    }

    #[test]
    fn panic_unwind() {
        let mut counter = Counter::default();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            counter.inc_n(10, |c| {
                assert!(c.value < 5);
                Ok(())
            })
        }));
        assert!(result.is_err());
        assert_eq!(counter.value, 0);
        assert_eq!(counter.max_value_seen, 5);
    }

    #[test]
    fn commit() {
        let mut counter = Counter::default();
        let result = counter.inc_n(10, |_| Ok(()));
        assert!(result.is_ok());
        assert_eq!(counter.value, 10);
        assert_eq!(counter.max_value_seen, 10);
    }
}
