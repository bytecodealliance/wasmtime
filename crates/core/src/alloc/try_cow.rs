use super::*;
use core::{cmp, fmt, hash, ops::Deref};
use std_alloc::borrow::Borrow;

/// Like [`std::borrow::ToOwned`] but returns an [`OutOfMemory`] error on
/// allocation failure.
pub trait TryToOwned {
    /// The owned version of this type.
    type Owned: Borrow<Self>;

    /// Try to allocate an owned version of `self`.
    fn try_to_owned(&self) -> Result<Self::Owned, OutOfMemory>;
}

impl TryToOwned for str {
    type Owned = String;

    fn try_to_owned(&self) -> Result<Self::Owned, OutOfMemory> {
        let mut s = String::new();
        s.push_str(self)?;
        Ok(s)
    }
}

impl<T> TryToOwned for [T]
where
    T: TryClone,
{
    type Owned = Vec<T>;

    fn try_to_owned(&self) -> Result<Self::Owned, OutOfMemory> {
        let mut v = Vec::with_capacity(self.len())?;
        for x in self {
            v.push(x.try_clone()?)?;
        }
        Ok(v)
    }
}

impl<T> TryToOwned for T
where
    T: TryClone,
{
    type Owned = Self;

    fn try_to_owned(&self) -> Result<Self::Owned, OutOfMemory> {
        self.try_clone()
    }
}

/// Like [`std::borrow::Cow`] but returns [`OutOfMemory`] errors for various
/// APIs that force allocation of an owned copy.
pub enum TryCow<'a, B>
where
    B: 'a + TryToOwned + ?Sized,
{
    /// Borrowed data.
    Borrowed(&'a B),

    /// Owned data.
    Owned(<B as TryToOwned>::Owned),
}

impl<'a, B> From<&'a B> for TryCow<'a, B>
where
    B: 'a + ?Sized + TryToOwned,
{
    fn from(b: &'a B) -> Self {
        Self::Borrowed(b)
    }
}

impl<B> Default for TryCow<'_, B>
where
    B: ?Sized + TryToOwned<Owned: Default>,
{
    fn default() -> Self {
        Self::Owned(<B as TryToOwned>::Owned::default())
    }
}

impl<B> fmt::Debug for TryCow<'_, B>
where
    B: ?Sized + fmt::Debug + TryToOwned<Owned: fmt::Debug>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Borrowed(b) => fmt::Debug::fmt(b, f),
            Self::Owned(o) => fmt::Debug::fmt(o, f),
        }
    }
}

impl<B> TryClone for TryCow<'_, B>
where
    B: ?Sized + TryToOwned,
{
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        match self {
            Self::Borrowed(b) => Ok(Self::Borrowed(b)),
            Self::Owned(o) => {
                let b: &B = o.borrow();
                Ok(Self::Owned(b.try_to_owned()?))
            }
        }
    }
}

impl<B> Deref for TryCow<'_, B>
where
    B: ?Sized + TryToOwned,
{
    type Target = B;

    fn deref(&self) -> &B {
        match self {
            Self::Borrowed(b) => b,
            Self::Owned(o) => o.borrow(),
        }
    }
}

impl<B> AsRef<B> for TryCow<'_, B>
where
    B: ?Sized + TryToOwned,
{
    fn as_ref(&self) -> &B {
        self
    }
}

impl<'a, B> Borrow<B> for TryCow<'a, B>
where
    B: ?Sized + TryToOwned,
{
    fn borrow(&self) -> &B {
        &**self
    }
}

impl<B> hash::Hash for TryCow<'_, B>
where
    B: ?Sized + hash::Hash + TryToOwned,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: hash::Hasher,
    {
        hash::Hash::hash(&**self, state)
    }
}

impl<B> PartialEq for TryCow<'_, B>
where
    B: ?Sized + PartialEq + TryToOwned,
{
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<B> Eq for TryCow<'_, B> where B: ?Sized + Eq + TryToOwned {}

impl<B> PartialOrd for TryCow<'_, B>
where
    B: ?Sized + PartialOrd + TryToOwned,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<B> Ord for TryCow<'_, B>
where
    B: ?Sized + Ord + TryToOwned,
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<'a, B> TryCow<'a, B>
where
    B: TryToOwned + ?Sized,
{
    /// Same as [`std::borrow::Cow::to_mut`] but returns an [`OutOfMemory`]
    /// error on allocation failure.
    pub fn to_mut(&mut self) -> Result<&mut <B as TryToOwned>::Owned, OutOfMemory> {
        if let Self::Borrowed(b) = self {
            *self = Self::Owned(b.try_to_owned()?);
        }
        match self {
            TryCow::Owned(x) => Ok(x),
            TryCow::Borrowed(_) => unreachable!(),
        }
    }

    /// Same as [`std::borrow::Cow::into_owned`] but returns an [`OutOfMemory`]
    /// error on allocation failure.
    pub fn into_owned(self) -> Result<<B as TryToOwned>::Owned, OutOfMemory> {
        match self {
            TryCow::Borrowed(b) => b.try_to_owned(),
            TryCow::Owned(x) => Ok(x),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;

    #[test]
    fn to_mut() -> Result<()> {
        let mut s = TryCow::Borrowed("hello");
        s.to_mut()?.push_str(", world!")?;
        assert!(matches!(s, TryCow::Owned(_)));
        assert_eq!(&*s, "hello, world!");
        Ok(())
    }

    #[test]
    fn into_owned() -> Result<()> {
        let v = TryCow::Borrowed(&[42u8, 36][..]);
        let v: Vec<u8> = v.into_owned()?;
        assert_eq!(&*v, &[42, 36]);
        Ok(())
    }
}
