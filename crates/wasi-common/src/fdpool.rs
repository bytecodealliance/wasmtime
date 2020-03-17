use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

pub(crate) trait Fd: Copy + Clone + Hash + PartialEq + Eq + Default {
    fn next(&self) -> Option<Self>;
}

pub(crate) struct FdPool<T: Fd> {
    next_alloc: Option<T>,
    available: Vec<T>,
    claimed: HashSet<T>,
}

impl<T: Fd + fmt::Debug> fmt::Debug for FdPool<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FdPool")
            .field("next_alloc", &self.next_alloc)
            .field("available", &self.available)
            .field("claimed", &self.claimed)
            .finish()
    }
}

impl<T: Fd> FdPool<T> {
    pub fn new() -> Self {
        Self {
            next_alloc: Some(T::default()),
            available: Vec::new(),
            claimed: HashSet::new(),
        }
    }

    pub fn allocate(&mut self) -> Option<T> {
        if let Some(fd) = self.available.pop() {
            // Since we've had free, unclaimed handle in the pool,
            // simply claim it and return.
            self.claimed.insert(fd);
            return Some(fd);
        }
        // There are no free handles available in the pool, so try
        // allocating an additional one into the pool. If we've
        // reached our max number of handles, we will fail with None
        // instead.
        let fd = match self.next_alloc.take() {
            None => return None,
            Some(fd) => fd,
        };
        // It's OK to not unpack the result of `fd.next()` here which
        // can fail since we check for `None` in the snippet above.
        self.next_alloc = fd.next();
        self.claimed.insert(fd);
        Some(fd)
    }

    pub fn deallocate(&mut self, fd: T) -> bool {
        if self.claimed.remove(&fd) {
            self.available.push(fd);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod test {
    use super::{Fd, FdPool};

    impl Fd for u8 {
        fn next(&self) -> Option<Self> {
            self.checked_add(1)
        }
    }

    #[test]
    fn basics() {
        let mut fdpool: FdPool<u8> = FdPool::new();
        let fd = fdpool.allocate().expect("success allocating 0");
        assert_eq!(fd, 0);
        let fd = fdpool.allocate().expect("success allocating 1");
        assert_eq!(fd, 1);
        let fd = fdpool.allocate().expect("success allocating 2");
        assert_eq!(fd, 2);
        fdpool.deallocate(1);
        fdpool.deallocate(0);
        let fd = fdpool.allocate().expect("success reallocating 0");
        assert_eq!(fd, 0);
        let fd = fdpool.allocate().expect("success reallocating 1");
        assert_eq!(fd, 1);
        let fd = fdpool.allocate().expect("success allocating 3");
        assert_eq!(fd, 3);
    }

    #[test]
    fn deallocate_nonexistent() {
        let mut fdpool: FdPool<u8> = FdPool::new();
        assert!(!fdpool.deallocate(0));
    }

    #[test]
    fn max_allocation() {
        let mut fdpool: FdPool<u8> = FdPool::new();
        for _ in 0..=std::u8::MAX {
            fdpool.allocate().expect("success allocating");
        }
        assert!(fdpool.allocate().is_none());
        assert!(fdpool.allocate().is_none());
        for i in 0..=std::u8::MAX {
            assert!(fdpool.deallocate(i));
        }
    }
}
