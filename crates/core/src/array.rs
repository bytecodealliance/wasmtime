use core::mem::MaybeUninit;

// See https://doc.rust-lang.org/core/array/fn.try_from_fn.html for the not yet stable original
//
/// Creates an array `[T; N]` where each fallible array element `T` is returned by the `cb` call.
/// Unlike [`from_fn`], where the element creation can't fail, this version will return an error
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
/// #![feature(array_try_from_fn)]
///
/// let array: Result<[u8; 5], _> = std::array::try_from_fn(|i| i.try_into());
/// assert_eq!(array, Ok([0, 1, 2, 3, 4]));
///
/// let array: Result<[i8; 200], _> = std::array::try_from_fn(|i| i.try_into());
/// assert!(array.is_err());
/// ```
//
// this is a reimplementation of array::try_from_fn, replace once that became stable
pub fn array_try_from_fn<E, F, T, const N: usize>(mut cb: F) -> Result<[T; N], E>
where
    F: FnMut(usize) -> Result<T, E>,
{
    let mut valid = 0;
    let mut result: [MaybeUninit<T>; N] = [const { MaybeUninit::uninit() }; N];
    let mut error: Option<E> = None;
    for n in 0..N {
        match cb(n) {
            Ok(v) => {
                result[valid].write(v);
                valid += 1;
            }
            Err(e) => {
                error = Some(e);
                // continue to consume all input
            }
        }
    }
    if let Some(e) = error {
        // on error drop all valid elements
        for n in 0..valid {
            unsafe {
                result[n].assume_init_drop();
            }
        }
        return Err(e);
    }
    assert!(valid == N);
    // this is a copy of array_assume_init from stdlib to avoid requiring nightly
    Ok(unsafe { (&result as *const _ as *const [T; N]).read() })
}

#[cfg(test)]
mod test {
    use super::array_try_from_fn;
    #[test]
    fn array_try_from_fn_test() {
        let array: Result<[u8; 5], _> = array_try_from_fn(|i| i.try_into());
        assert_eq!(array, Ok([0, 1, 2, 3, 4]));

        let array: Result<[i8; 200], _> = array_try_from_fn(|i| i.try_into());
        assert!(array.is_err());
    }
}
