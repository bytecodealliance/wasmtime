//! Contains mechanism for managing the WASI file descriptor
//! pool. It's intended to be mainly used within the `WasiCtx`
//! object(s).

/// Any type wishing to be treated as a valid WASI file descriptor
/// should implement this trait.
///
/// This trait is required as internally we use `u32` to represent
/// and manage raw file descriptors.
pub(crate) trait Fd {
    /// Convert to `u32`.
    fn as_raw(&self) -> u32;
    /// Convert from `u32`.
    fn from_raw(raw_fd: u32) -> Self;
}

impl Fd for u32 {
    fn as_raw(&self) -> u32 {
        *self
    }
    fn from_raw(raw_fd: u32) -> Self {
        raw_fd
    }
}

/// This container tracks and manages all file descriptors that
/// were already allocated.
/// Internally, we use `u32` to represent the file descriptors;
/// however, the caller may supply any type `T` such that it
/// implements the `Fd` trait when requesting a new descriptor
/// via the `allocate` method, or when returning one back via
/// the `deallocate` method.
#[derive(Debug)]
pub(crate) struct FdPool {
    next_alloc: Option<u32>,
    available: Vec<u32>,
}

impl FdPool {
    pub fn new() -> Self {
        Self {
            next_alloc: Some(0),
            available: Vec::new(),
        }
    }

    /// Obtain another valid WASI file descriptor.
    ///
    /// If we've handed out the maximum possible amount of file
    /// descriptors (which would be equal to `2^32 + 1` accounting for `0`),
    /// then this method will return `None` to signal that case.
    /// Otherwise, a new file descriptor is return as `Some(fd)`.
    pub fn allocate<T: Fd>(&mut self) -> Option<T> {
        if let Some(fd) = self.available.pop() {
            // Since we've had free, unclaimed handle in the pool,
            // simply claim it and return.
            return Some(T::from_raw(fd));
        }
        // There are no free handles available in the pool, so try
        // allocating an additional one into the pool. If we've
        // reached our max number of handles, we will fail with None
        // instead.
        let fd = self.next_alloc.take()?;
        // It's OK to not unpack the result of `fd.checked_add()` here which
        // can fail since we check for `None` in the snippet above.
        self.next_alloc = fd.checked_add(1);
        Some(T::from_raw(fd))
    }

    /// Return a file descriptor back to the pool.
    ///
    /// If the caller tries to return a file descriptor that was
    /// not yet allocated (via spoofing, etc.), this method
    /// will panic.
    pub fn deallocate<T: Fd>(&mut self, fd: T) {
        let fd = fd.as_raw();
        if let Some(next_alloc) = self.next_alloc {
            assert!(fd < next_alloc);
        }
        debug_assert!(!self.available.contains(&fd));
        self.available.push(fd);
    }
}

#[cfg(test)]
mod test {
    use super::FdPool;
    use std::ops::Deref;

    #[derive(Debug)]
    struct Fd(u32);

    impl super::Fd for Fd {
        fn as_raw(&self) -> u32 {
            self.0
        }
        fn from_raw(raw_fd: u32) -> Self {
            Self(raw_fd)
        }
    }

    impl Deref for Fd {
        type Target = u32;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[test]
    fn basics() {
        let mut fd_pool = FdPool::new();
        let mut fd: Fd = fd_pool.allocate().expect("success allocating 0");
        assert_eq!(*fd, 0);
        fd = fd_pool.allocate().expect("success allocating 1");
        assert_eq!(*fd, 1);
        fd = fd_pool.allocate().expect("success allocating 2");
        assert_eq!(*fd, 2);
        fd_pool.deallocate(1u32);
        fd_pool.deallocate(0u32);
        fd = fd_pool.allocate().expect("success reallocating 0");
        assert_eq!(*fd, 0);
        fd = fd_pool.allocate().expect("success reallocating 1");
        assert_eq!(*fd, 1);
        fd = fd_pool.allocate().expect("success allocating 3");
        assert_eq!(*fd, 3);
    }

    #[test]
    #[should_panic]
    fn deallocate_nonexistent() {
        let mut fd_pool = FdPool::new();
        fd_pool.deallocate(0u32);
    }

    #[test]
    fn max_allocation() {
        let mut fd_pool = FdPool::new();
        // Spoof reaching the limit of allocs.
        fd_pool.next_alloc = None;
        assert!(fd_pool.allocate::<Fd>().is_none());
    }
}
