use std::cmp::*;
use std::collections::HashMap;
use std::ops::Range;

/// Memory checker for wasm guest.
pub struct Wmemcheck {
    metadata: Vec<MemState>,
    mallocs: HashMap<usize, usize>,
    pub stack_pointer: usize,
    max_stack_size: usize,
    pub flag: bool,
    /// granularity in bytes of tracked allocations
    pub granularity: usize,
    pub enforce_uninitialized_reads: bool,
}

/// Error types for memory checker.
#[derive(Debug, PartialEq)]
pub enum AccessError {
    /// Malloc over already malloc'd memory.
    DoubleMalloc { addr: usize, len: usize },
    /// Read from uninitialized or undefined memory.
    InvalidRead { addr: usize, len: usize },
    /// Write to uninitialized memory.
    InvalidWrite { addr: usize, len: usize },
    /// Free of non-malloc'd pointer.
    InvalidFree { addr: usize },
    /// Reallocation of non-malloc'd pointer
    InvalidRealloc { addr: usize },
    /// Access out of bounds of heap or stack.
    OutOfBounds { addr: usize, len: usize },
}

/// Memory state for memory checker.
#[derive(Debug, Clone, PartialEq)]
pub enum MemState {
    /// Unallocated memory.
    Unallocated,
    /// Initialized but undefined memory.
    ValidToWrite,
    /// Initialized and defined memory.
    ValidToReadWrite,
}

impl Wmemcheck {
    /// Initializes memory checker instance.
    // TODO: how to make this properly configurable?
    pub fn new(
        mem_size: usize,
        granularity: usize,
        enforce_uninitialized_reads: bool,
    ) -> Wmemcheck {
        // TODO: metadata could be shrunk when granularity is greater than 1
        let metadata = vec![MemState::Unallocated; mem_size];
        let mallocs = HashMap::new();
        Wmemcheck {
            metadata,
            mallocs,
            stack_pointer: 0,
            max_stack_size: 0,
            flag: true,
            granularity,
            enforce_uninitialized_reads,
        }
    }

    /// Updates memory checker memory state metadata when malloc is called.
    pub fn allocate(
        &mut self,
        addr: usize,
        len: usize,
        initialized: bool,
    ) -> Result<(), AccessError> {
        if !self.is_in_bounds_heap(addr, len) {
            return Err(AccessError::OutOfBounds {
                addr: addr,
                len: len,
            });
        }
        for i in self.granular_range(addr..addr + len) {
            match self.metadata[i] {
                MemState::ValidToWrite => {
                    return Err(AccessError::DoubleMalloc {
                        addr: addr,
                        len: len,
                    });
                }
                MemState::ValidToReadWrite => {
                    return Err(AccessError::DoubleMalloc {
                        addr: addr,
                        len: len,
                    });
                }
                _ => {}
            }
        }
        for i in self.granular_range(addr..addr + len) {
            self.metadata[i] = if initialized {
                MemState::ValidToReadWrite
            } else {
                MemState::ValidToWrite
            };
        }
        self.mallocs.insert(addr, len);
        Ok(())
    }

    pub fn realloc(
        &mut self,
        end_addr: usize,
        start_addr: usize,
        len: usize,
    ) -> Result<(), AccessError> {
        if start_addr == 0 {
            // If ptr is NULL, realloc() is identical to a call to malloc()
            return self.allocate(end_addr, len, false);
        }
        if !self.mallocs.contains_key(&start_addr) {
            return Err(AccessError::InvalidRealloc { addr: start_addr });
        }
        let start_len = self.mallocs[&start_addr];
        // Copy initialization information from old allocation to new one
        let copy_len = start_len.min(len);
        let mut copied_metadata: Vec<MemState> = vec![];
        copied_metadata.extend_from_slice(&self.metadata[start_addr..start_addr + copy_len]);
        self.free(start_addr)?;
        self.allocate(end_addr, len, false)?;
        self.metadata[end_addr..end_addr + copy_len].clone_from_slice(&copied_metadata);
        Ok(())
    }

    /// Updates memory checker memory state metadata when a load occurs.
    pub fn read(&mut self, addr: usize, len: usize) -> Result<(), AccessError> {
        if !self.flag {
            return Ok(());
        }
        if !(self.is_in_bounds_stack(addr, len) || self.is_in_bounds_heap(addr, len)) {
            return Err(AccessError::OutOfBounds {
                addr: addr,
                len: len,
            });
        }
        for i in addr..addr + len {
            match self.metadata[i] {
                MemState::Unallocated => {
                    return Err(AccessError::InvalidRead {
                        addr: addr,
                        len: len,
                    });
                }
                MemState::ValidToWrite => {
                    if self.enforce_uninitialized_reads {
                        return Err(AccessError::InvalidRead {
                            addr: addr,
                            len: len,
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Updates memory checker memory state metadata when a store occurs.
    pub fn write(&mut self, addr: usize, len: usize) -> Result<(), AccessError> {
        if !self.flag {
            return Ok(());
        }
        if !(self.is_in_bounds_stack(addr, len) || self.is_in_bounds_heap(addr, len)) {
            return Err(AccessError::OutOfBounds {
                addr: addr,
                len: len,
            });
        }
        for i in self.granular_range(addr..addr + len) {
            if let MemState::Unallocated = self.metadata[i] {
                return Err(AccessError::InvalidWrite {
                    addr: addr,
                    len: len,
                });
            }
        }
        for i in self.granular_range(addr..addr + len) {
            self.metadata[i] = MemState::ValidToReadWrite;
        }
        Ok(())
    }

    /// Updates memory checker memory state metadata when free is called.
    pub fn free(&mut self, addr: usize) -> Result<(), AccessError> {
        if addr == 0 {
            return Ok(());
        }
        if !self.mallocs.contains_key(&addr) {
            return Err(AccessError::InvalidFree { addr: addr });
        }
        let len = self.mallocs[&addr];
        for i in self.granular_range(addr..addr + len) {
            if let MemState::Unallocated = self.metadata[i] {
                return Err(AccessError::InvalidFree { addr: addr });
            }
        }
        self.mallocs.remove(&addr);
        for i in self.granular_range(addr..addr + len) {
            self.metadata[i] = MemState::Unallocated;
        }
        Ok(())
    }

    fn is_in_bounds_heap(&self, addr: usize, len: usize) -> bool {
        self.max_stack_size <= addr && addr + len <= self.metadata.len()
    }

    fn is_in_bounds_stack(&self, addr: usize, len: usize) -> bool {
        self.stack_pointer <= addr && addr + len < self.max_stack_size
    }

    /// Updates memory checker metadata when stack pointer is updated.
    pub fn update_stack_pointer(&mut self, new_sp: usize) -> Result<(), AccessError> {
        if new_sp > self.max_stack_size {
            return Err(AccessError::OutOfBounds {
                addr: self.stack_pointer,
                len: new_sp - self.stack_pointer,
            });
        } else if new_sp < self.stack_pointer {
            for i in self.granular_range(new_sp..self.stack_pointer + 1) {
                self.metadata[i] = MemState::ValidToReadWrite;
            }
        } else {
            for i in self.granular_range(self.stack_pointer..new_sp) {
                self.metadata[i] = MemState::Unallocated;
            }
        }
        self.stack_pointer = new_sp;
        Ok(())
    }

    /// Turns memory checking on.
    pub fn memcheck_on(&mut self) {
        self.flag = true;
    }

    /// Turns memory checking off.
    pub fn memcheck_off(&mut self) {
        self.flag = false;
    }

    /// Initializes stack and stack pointer in memory checker metadata.
    pub fn set_stack_size(&mut self, stack_size: usize) {
        self.max_stack_size = stack_size + 1;
        // TODO: temporary solution to initialize the entire stack
        // while keeping stack tracing plumbing in place
        self.stack_pointer = stack_size;
        let _ = self.update_stack_pointer(0);
    }

    /// Updates memory checker metadata size when memory.grow is called.
    pub fn update_mem_size(&mut self, num_bytes: usize) {
        let to_append = vec![MemState::Unallocated; num_bytes];
        self.metadata.extend(to_append);
    }

    fn granular_range(&self, byte_range: Range<usize>) -> Range<usize> {
        // Round start of range down to granularity
        let start = (byte_range.start / self.granularity) * self.granularity;
        // Round end of range up to granularity
        let end = ((byte_range.end + self.granularity - 1) / self.granularity) * self.granularity;
        start..end
    }
}

#[test]
fn basic_wmemcheck() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    wmemcheck_state.set_stack_size(1024);
    assert!(wmemcheck_state.allocate(0x1000, 32, false).is_ok());
    assert!(wmemcheck_state.write(0x1000, 4).is_ok());
    assert!(wmemcheck_state.read(0x1000, 4).is_ok());
    assert_eq!(wmemcheck_state.mallocs, HashMap::from([(0x1000, 32)]));
    assert!(wmemcheck_state.free(0x1000).is_ok());
    assert!(wmemcheck_state.mallocs.is_empty());
}

#[test]
fn read_before_initializing() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    assert!(wmemcheck_state.allocate(0x1000, 32, false).is_ok());
    assert_eq!(
        wmemcheck_state.read(0x1000, 4),
        Err(AccessError::InvalidRead {
            addr: 0x1000,
            len: 4
        })
    );
    assert!(wmemcheck_state.write(0x1000, 4).is_ok());
    assert!(wmemcheck_state.free(0x1000).is_ok());
}

#[test]
fn use_after_free() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    assert!(wmemcheck_state.allocate(0x1000, 32, false).is_ok());
    assert!(wmemcheck_state.write(0x1000, 4).is_ok());
    assert!(wmemcheck_state.write(0x1000, 4).is_ok());
    assert!(wmemcheck_state.free(0x1000).is_ok());
    assert_eq!(
        wmemcheck_state.write(0x1000, 4),
        Err(AccessError::InvalidWrite {
            addr: 0x1000,
            len: 4
        })
    );
}

#[test]
fn double_free() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    assert!(wmemcheck_state.allocate(0x1000, 32, false).is_ok());
    assert!(wmemcheck_state.write(0x1000, 4).is_ok());
    assert!(wmemcheck_state.free(0x1000).is_ok());
    assert_eq!(
        wmemcheck_state.free(0x1000),
        Err(AccessError::InvalidFree { addr: 0x1000 })
    );
}

#[test]
fn out_of_bounds_malloc() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    assert_eq!(
        wmemcheck_state.allocate(640 * 1024, 1, false),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024,
            len: 1
        })
    );
    assert_eq!(
        wmemcheck_state.allocate(640 * 1024 - 10, 15, false),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024 - 10,
            len: 15
        })
    );
    assert!(wmemcheck_state.mallocs.is_empty());
}

#[test]
fn out_of_bounds_read() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    assert!(wmemcheck_state.allocate(640 * 1024 - 24, 24, false).is_ok());
    assert_eq!(
        wmemcheck_state.read(640 * 1024 - 24, 25),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024 - 24,
            len: 25
        })
    );
}

#[test]
fn double_malloc() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    assert!(wmemcheck_state.allocate(0x1000, 32, false).is_ok());
    assert_eq!(
        wmemcheck_state.allocate(0x1000, 32, false),
        Err(AccessError::DoubleMalloc {
            addr: 0x1000,
            len: 32
        })
    );
    assert_eq!(
        wmemcheck_state.allocate(0x1002, 32, false),
        Err(AccessError::DoubleMalloc {
            addr: 0x1002,
            len: 32
        })
    );
    assert!(wmemcheck_state.free(0x1000).is_ok());
}

#[test]
fn error_type() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    assert!(wmemcheck_state.allocate(0x1000, 32, false).is_ok());
    assert_eq!(
        wmemcheck_state.allocate(0x1000, 32, false),
        Err(AccessError::DoubleMalloc {
            addr: 0x1000,
            len: 32
        })
    );
    assert_eq!(
        wmemcheck_state.allocate(640 * 1024, 32, false),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024,
            len: 32
        })
    );
    assert!(wmemcheck_state.free(0x1000).is_ok());
}

#[test]
fn update_sp_no_error() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    wmemcheck_state.set_stack_size(1024);
    assert!(wmemcheck_state.update_stack_pointer(768).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 768);
    assert!(wmemcheck_state.allocate(1024 * 2, 32, false).is_ok());
    assert!(wmemcheck_state.free(1024 * 2).is_ok());
    assert!(wmemcheck_state.update_stack_pointer(896).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 896);
    assert!(wmemcheck_state.update_stack_pointer(1024).is_ok());
}

#[test]
fn bad_stack_malloc() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    wmemcheck_state.set_stack_size(1024);

    assert!(wmemcheck_state.update_stack_pointer(0).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 0);
    assert_eq!(
        wmemcheck_state.allocate(512, 32, false),
        Err(AccessError::OutOfBounds { addr: 512, len: 32 })
    );
    assert_eq!(
        wmemcheck_state.allocate(1022, 32, false),
        Err(AccessError::OutOfBounds {
            addr: 1022,
            len: 32
        })
    );
}

#[test]
fn stack_full_empty() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024, 1, true);

    wmemcheck_state.set_stack_size(1024);

    assert!(wmemcheck_state.update_stack_pointer(0).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 0);
    assert!(wmemcheck_state.update_stack_pointer(1024).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 1024)
}

#[test]
fn from_test_program() {
    let mut wmemcheck_state = Wmemcheck::new(1024 * 1024 * 128, 1, true);
    wmemcheck_state.set_stack_size(70864);
    assert!(wmemcheck_state.write(70832, 1).is_ok());
    assert!(wmemcheck_state.read(1138, 1).is_ok());
}
