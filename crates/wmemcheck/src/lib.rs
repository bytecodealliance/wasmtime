use std::cmp::*;
use std::collections::HashMap;

/// Memory checker for wasm guest.
pub struct Wmemcheck {
    metadata: Vec<MemState>,
    mallocs: HashMap<usize, usize>,
    pub stack_pointer: usize,
    max_stack_size: usize,
    pub flag: bool,
}

impl Wmemcheck {
    pub fn malloc_previous_to(&self, addr: usize) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize)> = None;
        for (base, len) in self.mallocs.iter() {
            if let Some((prev_base, _)) = best {
                if prev_base < *base && *base <= addr {
                    best = Some((*base, *len));
                }
            } else {
                best = Some((*base, *len));
            }
        }
        best
    }
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
    pub fn new(mem_size: usize) -> Wmemcheck {
        let metadata = vec![MemState::Unallocated; mem_size];
        let mallocs = HashMap::new();
        Wmemcheck {
            metadata,
            mallocs,
            stack_pointer: 0,
            max_stack_size: 0,
            flag: true,
        }
    }

    /// Updates memory checker memory state metadata when malloc is called.
    pub fn malloc(&mut self, addr: usize, start_len: usize) -> Result<(), AccessError> {
        // round up to multiple of 4
        let len = (start_len + 3) & !3;

        if !self.is_in_bounds_heap(addr, len) {
            return Err(AccessError::OutOfBounds {
                addr: addr,
                len: len,
            });
        }
        for i in addr..addr + len {
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
        for i in addr..addr + len {
            self.metadata[i] = MemState::ValidToWrite;
        }
        self.mallocs.insert(addr, len);
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
                /* MemState::ValidToWrite => {
                    return Err(AccessError::InvalidRead {
                        addr: addr,
                        len: len,
                    });
                } */
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
        for i in addr..addr + len {
            if let MemState::Unallocated = self.metadata[i] {
                return Err(AccessError::InvalidWrite {
                    addr: addr,
                    len: len,
                });
            }
        }
        for i in addr..addr + len {
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
        for i in addr..addr + len {
            if let MemState::Unallocated = self.metadata[i] {
                return Err(AccessError::InvalidFree { addr: addr });
            }
        }
        self.mallocs.remove(&addr);
        for i in addr..addr + len {
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
            for i in new_sp..self.stack_pointer + 1 {
                self.metadata[i] = MemState::ValidToReadWrite;
            }
        } else {
            for i in self.stack_pointer..new_sp {
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
}

#[test]
fn basic_wmemcheck() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    wmemcheck_state.set_stack_size(1024);
    assert!(wmemcheck_state.malloc(0x1000, 32).is_ok());
    assert!(wmemcheck_state.write(0x1000, 4).is_ok());
    assert!(wmemcheck_state.read(0x1000, 4).is_ok());
    assert_eq!(wmemcheck_state.mallocs, HashMap::from([(0x1000, 32)]));
    assert!(wmemcheck_state.free(0x1000).is_ok());
    assert!(wmemcheck_state.mallocs.is_empty());
}

#[test]
fn read_before_initializing() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    assert!(wmemcheck_state.malloc(0x1000, 32).is_ok());
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
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    assert!(wmemcheck_state.malloc(0x1000, 32).is_ok());
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
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    assert!(wmemcheck_state.malloc(0x1000, 32).is_ok());
    assert!(wmemcheck_state.write(0x1000, 4).is_ok());
    assert!(wmemcheck_state.free(0x1000).is_ok());
    assert_eq!(
        wmemcheck_state.free(0x1000),
        Err(AccessError::InvalidFree { addr: 0x1000 })
    );
}

#[test]
fn out_of_bounds_malloc() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    assert_eq!(
        wmemcheck_state.malloc(640 * 1024, 1),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024,
            len: 1
        })
    );
    assert_eq!(
        wmemcheck_state.malloc(640 * 1024 - 10, 15),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024 - 10,
            len: 15
        })
    );
    assert!(wmemcheck_state.mallocs.is_empty());
}

#[test]
fn out_of_bounds_read() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    assert!(wmemcheck_state.malloc(640 * 1024 - 24, 24).is_ok());
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
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    assert!(wmemcheck_state.malloc(0x1000, 32).is_ok());
    assert_eq!(
        wmemcheck_state.malloc(0x1000, 32),
        Err(AccessError::DoubleMalloc {
            addr: 0x1000,
            len: 32
        })
    );
    assert_eq!(
        wmemcheck_state.malloc(0x1002, 32),
        Err(AccessError::DoubleMalloc {
            addr: 0x1002,
            len: 32
        })
    );
    assert!(wmemcheck_state.free(0x1000).is_ok());
}

#[test]
fn error_type() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    assert!(wmemcheck_state.malloc(0x1000, 32).is_ok());
    assert_eq!(
        wmemcheck_state.malloc(0x1000, 32),
        Err(AccessError::DoubleMalloc {
            addr: 0x1000,
            len: 32
        })
    );
    assert_eq!(
        wmemcheck_state.malloc(640 * 1024, 32),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024,
            len: 32
        })
    );
    assert!(wmemcheck_state.free(0x1000).is_ok());
}

#[test]
fn update_sp_no_error() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    wmemcheck_state.set_stack_size(1024);
    assert!(wmemcheck_state.update_stack_pointer(768).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 768);
    assert!(wmemcheck_state.malloc(1024 * 2, 32).is_ok());
    assert!(wmemcheck_state.free(1024 * 2).is_ok());
    assert!(wmemcheck_state.update_stack_pointer(896).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 896);
    assert!(wmemcheck_state.update_stack_pointer(1024).is_ok());
}

#[test]
fn bad_stack_malloc() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    wmemcheck_state.set_stack_size(1024);

    assert!(wmemcheck_state.update_stack_pointer(0).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 0);
    assert_eq!(
        wmemcheck_state.malloc(512, 32),
        Err(AccessError::OutOfBounds { addr: 512, len: 32 })
    );
    assert_eq!(
        wmemcheck_state.malloc(1022, 32),
        Err(AccessError::OutOfBounds {
            addr: 1022,
            len: 32
        })
    );
}

#[test]
fn stack_full_empty() {
    let mut wmemcheck_state = Wmemcheck::new(640 * 1024);

    wmemcheck_state.set_stack_size(1024);

    assert!(wmemcheck_state.update_stack_pointer(0).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 0);
    assert!(wmemcheck_state.update_stack_pointer(1024).is_ok());
    assert_eq!(wmemcheck_state.stack_pointer, 1024)
}

#[test]
fn from_test_program() {
    let mut wmemcheck_state = Wmemcheck::new(1024 * 1024 * 128);
    wmemcheck_state.set_stack_size(70864);
    assert!(wmemcheck_state.write(70832, 1).is_ok());
    assert!(wmemcheck_state.read(1138, 1).is_ok());
}
