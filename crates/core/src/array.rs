//! Not yet stable array functions needed by fixed length list

use core::mem::MaybeUninit;

// See https://doc.rust-lang.org/core/array/fn.try_from_fn.html for the not yet stable original
//
/// Creates an array `[T; N]` where each fallible array element `T` is returned by the `cb` call.
/// Unlike [`core::array::from_fn`], where the element creation can't fail, this version will return an error
/// if any element creation was unsuccessful.
///
/// The return type of this function depends on the return type of the closure.
/// If you return `Result<T, E>` from the closure, you'll get a `Result<[T; N], E>`.
///
/// Note: Unlike the unstable core implementation this function only supports a closure returning a Result.
///
/// # Arguments
///
/// * `cb`: Callback where the passed argument is the current array index.
///
/// # Example
///
/// ```rust
/// use wasmtime_internal_core::array::array_try_from_fn;
///
/// let array: Result<[u8; 5], _> = array_try_from_fn(|i| i.try_into());
/// assert_eq!(array, Ok([0, 1, 2, 3, 4]));
///
/// let array: Result<[i8; 200], _> = array_try_from_fn(|i| i.try_into());
/// assert!(array.is_err());
/// ```
//
// this is a reimplementation of array::try_from_fn, replace once that became stable
pub fn array_try_from_fn<E, T, const N: usize>(
    mut cb: impl FnMut(usize) -> Result<T, E>,
) -> Result<[T; N], E> {
    let mut result: MaybeUninit<[T; N]> = MaybeUninit::uninit();
    {
        struct DropGuard<'a, T> {
            slice: &'a mut [MaybeUninit<T>],
            initialized: usize,
        }
        impl<T> Drop for DropGuard<'_, T> {
            fn drop(&mut self) {
                for slot in self.slice[..self.initialized].iter_mut() {
                    // SAFETY: self.initialized is the number of valid elements at all time
                    // we can assume init and drop them directly
                    unsafe {
                        slot.assume_init_drop();
                    }
                }
            }
        }
        let mut guard = DropGuard {
            // Note .as_mut() would be ideal but requires 1.95, so we work around it
            // SAFETY: We cast from an uninitialized ptr to an array of T to an array of uninitialized T
            // The bit pattern is identical, and unstable provides the safe function transpose for this
            // Turning a pointer to uninitialized memory into a mutable reference to uninitialized memory is a valid op
            slice: unsafe { &mut *(result.as_mut_ptr().cast::<[MaybeUninit<T>; N]>()) },
            initialized: 0,
        };
        for (i, slot) in guard.slice.iter_mut().enumerate() {
            slot.write(cb(i)?);
            guard.initialized = i + 1;
        }
        // don't drop valid elements
        guard.initialized = 0;
    }
    // SAFETY: All N elements have been successfully written to here
    unsafe { Ok(result.assume_init()) }
}

#[cfg(test)]
mod test {
    use super::array_try_from_fn;
    use core::cell::Cell;
    use std_alloc::rc::Rc;
    use std_alloc::string::{String, ToString};

    // original test from the documentation
    #[test]
    fn array_try_from_fn_test() {
        let array: Result<[u8; 5], _> = array_try_from_fn(|i| i.try_into());
        assert_eq!(array, Ok([0, 1, 2, 3, 4]));

        let array: Result<[i8; 200], _> = array_try_from_fn(|i| i.try_into());
        assert!(array.is_err());
    }

    #[test]
    fn smoke_try_from_fn() {
        let arr = array_try_from_fn(|i| Ok::<_, ()>(i * 2)).unwrap();
        assert_eq!(arr, [0, 2, 4, 6, 8]);
        assert_eq!(
            array_try_from_fn::<_, _, 3>(|i| if i == 0 { Ok(0) } else { Err(1) }).unwrap_err(),
            1
        )
    }

    #[test]
    fn try_from_fn_dont_drop_on_success() {
        let arr = array_try_from_fn(|i| Ok::<_, String>(i.to_string())).unwrap();
        assert_eq!(arr, ["0", "1"]);
    }

    #[test]
    fn try_from_fn_drop_on_failure() {
        let drops = Rc::new(Cell::new(0));

        struct DropCounter(Rc<Cell<usize>>);
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let err = array_try_from_fn::<_, _, 10>(|i| match i {
            0..=4 => Ok(DropCounter(drops.clone())),
            _ => Err("error".to_string()),
        })
        .err()
        .unwrap();
        assert_eq!(err, "error");
        assert_eq!(drops.get(), 5);
    }

    #[test]
    #[cfg(feature = "std")]
    fn try_from_fn_drop_on_panic() {
        let drops = Rc::new(Cell::new(0));

        struct DropCounter(Rc<Cell<usize>>);
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            array_try_from_fn::<_, _, 10>(|i| match i {
                0..=4 => Ok::<_, String>(DropCounter(drops.clone())),
                _ => panic!("hi"),
            })
        }));
        assert!(result.is_err());
        assert_eq!(drops.get(), 5);
    }
}
