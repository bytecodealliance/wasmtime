use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::hash::Hash;

pub(crate) trait Fd: Copy + Clone + Hash + PartialEq + Eq + Default {
    fn next(&self) -> Option<Self>;
}

pub(crate) struct FdSet<T: Fd> {
    available: VecDeque<T>,
    claimed: HashSet<T>,
}

impl<T: Fd + fmt::Debug> fmt::Debug for FdSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FdSet")
            .field("available", &self.available)
            .field("claimed", &self.claimed)
            .finish()
    }
}

impl<T: Fd> FdSet<T> {
    const BATCH_SIZE: usize = 64;

    pub fn new() -> Self {
        // Try preallocating `BATCH_SIZE` worth of fds. If we're
        // unable to do so, preallocated as many as we can.
        let mut available = VecDeque::with_capacity(Self::BATCH_SIZE);
        Self::preallocate(T::default(), &mut available);

        Self {
            available,
            claimed: HashSet::new(),
        }
    }

    pub fn allocate(&mut self) -> Option<T> {
        // When popping from the stack, we always pop from the back
        // so that the "largest" fd value stays at the bottom.
        let fd = match self.available.pop_back() {
            None => return None,
            Some(fd) => fd,
        };
        // Before popping from the stack, check if available pool is nonempty.
        // If so, try preallocating another `BATCH_SIZE` worth of fds.
        // If that fails, then we have reached our max number of
        // allocations and will fail at next attempt, unless
        // some values are freed first.
        if self.available.is_empty() {
            self.available.reserve(Self::BATCH_SIZE);
            if let Some(fd) = fd.next() {
                Self::preallocate(fd, &mut self.available);
            }
        }
        // Afterwards, claim the popped value.
        self.claimed.insert(fd);
        Some(fd)
    }

    pub fn deallocate(&mut self, fd: T) -> bool {
        if self.claimed.remove(&fd) {
            self.available.push_back(fd);
            return true;
        }
        false
    }

    fn preallocate(mut val: T, vals: &mut VecDeque<T>) {
        // When preallocating, we always push front so that we
        // always end up with the "largest" possible fd value at the
        // bottom of the stack.
        //
        // Note that this may end up not allocating a single value.
        for _ in 0..Self::BATCH_SIZE {
            vals.push_front(val);
            match val.next() {
                Some(v) => val = v,
                None => break,
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Fd, FdSet};

    impl Fd for u8 {
        fn next(&self) -> Option<Self> {
            self.checked_add(1)
        }
    }

    #[test]
    fn basics() {
        let mut fdset: FdSet<u8> = FdSet::new();
        let fd = fdset.allocate().expect("success allocating 0");
        assert_eq!(fd, 0);
        println!("{:?}", fdset);
        let fd = fdset.allocate().expect("success allocating 1");
        assert_eq!(fd, 1);
        println!("{:?}", fdset);
        let fd = fdset.allocate().expect("success allocating 2");
        assert_eq!(fd, 2);
        println!("{:?}", fdset);
        fdset.deallocate(1);
        println!("{:?}", fdset);
        fdset.deallocate(0);
        println!("{:?}", fdset);
        let fd = fdset.allocate().expect("success reallocating 0");
        assert_eq!(fd, 0);
        let fd = fdset.allocate().expect("success reallocating 1");
        assert_eq!(fd, 1);
        let fd = fdset.allocate().expect("success allocating 3");
        assert_eq!(fd, 3);
    }

    #[test]
    fn deallocate_nonexistent() {
        let mut fdset: FdSet<u8> = FdSet::new();
        assert!(!fdset.deallocate(0));
    }

    #[test]
    fn max_allocation() {
        let mut fdset: FdSet<u8> = FdSet::new();
        for _ in 0..=std::u8::MAX {
            fdset.allocate().expect("success allocating");
        }
        assert!(fdset.allocate().is_none());
        assert!(fdset.allocate().is_none());
        for i in 0..=std::u8::MAX {
            assert!(fdset.deallocate(i));
        }
    }
}
