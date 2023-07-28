/*
The following implementation assumes that the stack sits at the bottom of memory.
*/

use std::cmp::*;
use std::collections::HashMap;

pub struct Valgrind {
    metadata: Vec<MemState>,
    mallocs: HashMap<usize, usize>, // start addr, len
    pub stack_pointer: usize,
    max_stack_size: usize,
    pub flag: bool,
}

#[derive(Debug, PartialEq)]
pub enum AccessError {
    DoubleMalloc { addr: usize, len: usize },
    InvalidRead { addr: usize, len: usize },
    InvalidWrite { addr: usize, len: usize },
    InvalidFree { addr: usize },
    OutOfBounds { addr: usize, len: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemState {
    Unallocated,
    ValidToWrite,
    ValidToReadWrite,
}

impl Valgrind {
    pub fn new(mem_size: usize) -> Valgrind {
        let metadata = vec![MemState::Unallocated; mem_size];
        let mallocs = HashMap::new();
        Valgrind {
            metadata,
            mallocs,
            stack_pointer: 0,
            max_stack_size: 0,
            flag: true,
        }
    }
    pub fn malloc(&mut self, addr: usize, len: usize) -> Result<(), AccessError> {
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
                    return Err(AccessError::InvalidRead {
                        addr: addr,
                        len: len,
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }
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
    pub fn free(&mut self, addr: usize) -> Result<(), AccessError> {
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
    pub fn update_stack_pointer(&mut self, new_sp: usize) -> Result<(), AccessError> {
        if new_sp > self.max_stack_size {
            return Err(AccessError::OutOfBounds {
                addr: self.stack_pointer,
                len: new_sp - self.stack_pointer,
            });
        } else if new_sp < self.stack_pointer {
            for i in new_sp..self.stack_pointer + 1 {
                // +1 to account for sp == max_stack_size (?)
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
    pub fn memcheck_on(&mut self) {
        self.flag = true;
    }
    pub fn memcheck_off(&mut self) {
        self.flag = false;
    }
    pub fn set_stack_size(&mut self, stack_size: usize) {
        self.max_stack_size = stack_size + 1;
        //temporary solution to initialize the entire stack
        //while keeping stack tracing plumbing in place
        self.stack_pointer = stack_size;
        self.update_stack_pointer(0);
    }
}

#[test]
fn basic_valgrind() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 0);

    assert!(valgrind_state.malloc(0x1000, 32).is_ok());
    assert!(valgrind_state.write(0x1000, 4).is_ok());
    assert!(valgrind_state.read(0x1000, 4).is_ok());
    assert_eq!(valgrind_state.mallocs, HashMap::from([(0x1000, 32)]));
    assert!(valgrind_state.free(0x1000).is_ok());
    assert!(valgrind_state.mallocs.is_empty());
}

#[test]
fn read_before_initializing() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 0);

    assert!(valgrind_state.malloc(0x1000, 32).is_ok());
    assert_eq!(
        valgrind_state.read(0x1000, 4),
        Err(AccessError::InvalidRead {
            addr: 0x1000,
            len: 4
        })
    );
    assert!(valgrind_state.write(0x1000, 4).is_ok());
    assert!(valgrind_state.free(0x1000).is_ok());
}

#[test]
fn use_after_free() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 0);

    assert!(valgrind_state.malloc(0x1000, 32).is_ok());
    assert!(valgrind_state.write(0x1000, 4).is_ok());
    assert!(valgrind_state.write(0x1000, 4).is_ok());
    assert!(valgrind_state.free(0x1000).is_ok());
    assert_eq!(
        valgrind_state.write(0x1000, 4),
        Err(AccessError::InvalidWrite {
            addr: 0x1000,
            len: 4
        })
    );
}

#[test]
fn double_free() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 0);

    assert!(valgrind_state.malloc(0x1000, 32).is_ok());
    assert!(valgrind_state.write(0x1000, 4).is_ok());
    assert!(valgrind_state.free(0x1000).is_ok());
    assert_eq!(
        valgrind_state.free(0x1000),
        Err(AccessError::InvalidFree { addr: 0x1000 })
    );
}

#[test]
fn out_of_bounds_malloc() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 0);

    assert_eq!(
        valgrind_state.malloc(640 * 1024, 1),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024,
            len: 1
        })
    );
    assert_eq!(
        valgrind_state.malloc(640 * 1024 - 10, 15),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024 - 10,
            len: 15
        })
    );
    assert!(valgrind_state.mallocs.is_empty());
}

#[test]
fn out_of_bounds_read() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 0);

    assert!(valgrind_state.malloc(640 * 1024 - 24, 24).is_ok());
    assert_eq!(
        valgrind_state.read(640 * 1024 - 24, 25),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024 - 24,
            len: 25
        })
    );
}

#[test]
fn double_malloc() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 0);

    assert!(valgrind_state.malloc(0x1000, 32).is_ok());
    assert_eq!(
        valgrind_state.malloc(0x1000, 32),
        Err(AccessError::DoubleMalloc {
            addr: 0x1000,
            len: 32
        })
    );
    assert_eq!(
        valgrind_state.malloc(0x1002, 32),
        Err(AccessError::DoubleMalloc {
            addr: 0x1002,
            len: 32
        })
    );
    assert!(valgrind_state.free(0x1000).is_ok());
}

#[test]
fn error_type() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 0);

    assert!(valgrind_state.malloc(0x1000, 32).is_ok());
    assert_eq!(
        valgrind_state.malloc(0x1000, 32),
        Err(AccessError::DoubleMalloc {
            addr: 0x1000,
            len: 32
        })
    );
    assert_eq!(
        valgrind_state.malloc(640 * 1024, 32),
        Err(AccessError::OutOfBounds {
            addr: 640 * 1024,
            len: 32
        })
    );
    assert!(valgrind_state.free(0x1000).is_ok());
}

#[test]
fn update_sp_no_error() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 1024);

    assert_eq!(valgrind_state.max_stack_size, 1024);
    assert!(valgrind_state.update_stack_pointer(768).is_ok());
    assert_eq!(valgrind_state.stack_pointer, 768);
    assert!(valgrind_state.malloc(1024 * 2, 32).is_ok());
    assert!(valgrind_state.free(1024 * 2).is_ok());
    assert!(valgrind_state.update_stack_pointer(896).is_ok());
    assert_eq!(valgrind_state.stack_pointer, 896);
    assert!(valgrind_state.update_stack_pointer(1024).is_ok());
}

#[test]
fn bad_stack_malloc() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 1024);

    assert!(valgrind_state.update_stack_pointer(0).is_ok());
    assert_eq!(valgrind_state.stack_pointer, 0);
    assert_eq!(
        valgrind_state.malloc(512, 32),
        Err(AccessError::OutOfBounds { addr: 512, len: 32 })
    );
    assert_eq!(
        valgrind_state.malloc(1022, 32),
        Err(AccessError::OutOfBounds {
            addr: 1022,
            len: 32
        })
    );
}

#[test]
fn bad_stack_read_write() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 1024);

    assert!(valgrind_state.update_stack_pointer(512).is_ok());
    assert_eq!(valgrind_state.stack_pointer, 512);
    assert_eq!(
        valgrind_state.read(256, 16),
        Err(AccessError::InvalidRead { addr: 256, len: 16 })
    );
    assert_eq!(
        valgrind_state.write(500, 32),
        Err(AccessError::InvalidWrite { addr: 500, len: 32 })
    );
}

#[test]
fn stack_full_empty() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 1024);

    assert!(valgrind_state.update_stack_pointer(0).is_ok());
    assert_eq!(valgrind_state.stack_pointer, 0);
    assert!(valgrind_state.update_stack_pointer(1024).is_ok());
    assert_eq!(valgrind_state.stack_pointer, 1024)
}

#[test]
fn stack_underflow() {
    let mut valgrind_state = Valgrind::new(640 * 1024, 1024);

    assert!(valgrind_state.update_stack_pointer(800).is_ok());
    assert_eq!(
        valgrind_state.update_stack_pointer(1025),
        Err(AccessError::OutOfBounds {
            addr: 800,
            len: 225
        })
    );
    assert_eq!(
        valgrind_state.update_stack_pointer(2000),
        Err(AccessError::OutOfBounds {
            addr: 800,
            len: 1200
        })
    );
    assert_eq!(valgrind_state.stack_pointer, 800);
}

#[test]
fn from_test_program() {
    let mut valgrind_state = Valgrind::new(1024 * 1024 * 128, 70864);

    assert!(valgrind_state.write(70832, 1).is_ok());
    assert!(valgrind_state.read(1138, 1).is_ok());
}
