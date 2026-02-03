use crate::alloc::Vec;
use crate::error::OutOfMemory;
use std_alloc::boxed::Box;

/// Extension trait for an `Iterator` to fallibly collect into a container.
pub trait TryCollect: Iterator {
    /// Attempts to collect the iterator `self` into `B`.
    ///
    /// Same as [`Iterator::collect`] except returns OOM instead of aborting.
    fn try_collect<B, E>(self) -> Result<B, E>
    where
        B: TryFromIterator<Self::Item, E>,
        Self: Sized,
    {
        B::try_from_iter(self)
    }
}

impl<I: Iterator> TryCollect for I {}

/// Analogue of [`FromIterator`] in the standard library, but used with
/// [`TryCollect::try_collect`] instead.
pub trait TryFromIterator<T, E>: Sized {
    /// Creates an intance of this collection from the `iter` provided.
    ///
    /// Does not abort on OOM but insteads return an error.
    fn try_from_iter<I>(iter: I) -> Result<Self, E>
    where
        I: Iterator<Item = T>;
}

impl<T> TryFromIterator<T, OutOfMemory> for Vec<T> {
    fn try_from_iter<I>(iter: I) -> Result<Self, OutOfMemory>
    where
        I: Iterator<Item = T>,
    {
        let mut result = Vec::with_capacity(iter.size_hint().0)?;
        for item in iter {
            result.push(item)?;
        }
        Ok(result)
    }
}

impl<T> TryFromIterator<T, OutOfMemory> for Box<[T]> {
    fn try_from_iter<I>(iter: I) -> Result<Self, OutOfMemory>
    where
        I: Iterator<Item = T>,
    {
        let vec = Vec::try_from_iter(iter)?;
        vec.into_boxed_slice()
    }
}

impl<T, E> TryFromIterator<Result<T, E>, E> for Vec<T>
where
    E: From<OutOfMemory>,
{
    fn try_from_iter<I>(iter: I) -> Result<Self, E>
    where
        I: Iterator<Item = Result<T, E>>,
    {
        let mut result = Vec::with_capacity(iter.size_hint().0)?;
        for item in iter {
            result.push(item?)?;
        }
        Ok(result)
    }
}

impl<T, E> TryFromIterator<Result<T, E>, E> for Box<[T]>
where
    E: From<OutOfMemory>,
{
    fn try_from_iter<I>(iter: I) -> Result<Self, E>
    where
        I: Iterator<Item = Result<T, E>>,
    {
        let vec = iter.try_collect::<Vec<_>, E>()?;
        Ok(vec.into_boxed_slice()?)
    }
}

/// Analogue of [`Extend`] except handles OOM conditions.
pub trait TryExtend<T> {
    /// Extends `self` with the items from `iter`.
    ///
    /// Returns an error if allocation fails while adding items to `self`. If an
    /// OOM happens then some items from `iter` may have been added to `self`
    /// already. On OOM no further items from the iterator will be consumed.
    fn try_extend<I>(&mut self, iter: I) -> Result<(), OutOfMemory>
    where
        I: IntoIterator<Item = T>;
}

impl<T> TryExtend<T> for Vec<T> {
    fn try_extend<I>(&mut self, iter: I) -> Result<(), OutOfMemory>
    where
        I: IntoIterator<Item = T>,
    {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0)?;
        for item in iter {
            self.push(item)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Box, TryCollect, TryExtend, Vec};
    use crate::error::{OutOfMemory, Result};

    #[test]
    fn test_vec_collect() -> Result<(), OutOfMemory> {
        let v: Vec<i32> = (0..10).try_collect()?;
        assert_eq!(&*v, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        Ok(())
    }

    #[test]
    fn test_box_collect() -> Result<(), OutOfMemory> {
        let v: Box<[i32]> = (0..10).try_collect()?;
        assert_eq!(&*v, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        Ok(())
    }

    #[test]
    fn test_vec_result_collect() -> Result<()> {
        let v: Result<Vec<i32>> = [].into_iter().try_collect();
        assert!(v?.is_empty());

        let v: Result<Vec<i32>> = [Ok(1), Ok(2)].into_iter().try_collect();
        assert_eq!(&*v?, &[1, 2]);

        let v: Result<Vec<i32>> = [Ok(1), Err(crate::format_err!("hi"))]
            .into_iter()
            .try_collect();
        assert!(v.is_err());

        let v: Result<Vec<i32>> = [Err(crate::format_err!("hi")), Ok(1)]
            .into_iter()
            .try_collect();
        assert!(v.is_err());
        Ok(())
    }

    #[test]
    fn test_box_result_collect() -> Result<()> {
        let v: Result<Box<[i32]>> = [].into_iter().try_collect();
        assert!(v?.is_empty());

        let v: Result<Box<[i32]>> = [Ok(1), Ok(2)].into_iter().try_collect();
        assert_eq!(&*v?, &[1, 2]);

        let v: Result<Box<[i32]>> = [Ok(1), Err(crate::format_err!("hi"))]
            .into_iter()
            .try_collect();
        assert!(v.is_err());

        let v: Result<Box<[i32]>> = [Err(crate::format_err!("hi")), Ok(1)]
            .into_iter()
            .try_collect();
        assert!(v.is_err());
        Ok(())
    }

    #[test]
    fn test_try_extend() -> Result<(), OutOfMemory> {
        let mut vec = Vec::new();
        vec.try_extend([1, 2, 3].iter().cloned())?;
        assert_eq!(&*vec, &[1, 2, 3]);

        vec.try_extend([])?;
        assert_eq!(&*vec, &[1, 2, 3]);
        Ok(())
    }
}
