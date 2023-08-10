#![no_main]

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use wmemcheck::{Wmemcheck, MemState, AccessError};

const TEST_MAX_ADDR: usize = 1024 * 640 - 1;
const TEST_MAX_STACK_SIZE: usize = 1024;

fuzz_target!(|data: &[u8]| {
    let u = &mut Unstructured::new(data);
    let mut wmemcheck_state = Wmemcheck::new(TEST_MAX_ADDR + 1, TEST_MAX_STACK_SIZE);
    let cmds = match BuggyCommandSequence::arbitrary(u) {
        Ok(val) => val,
        Err(_) => return,
    };
    println!("commands: {:?}", cmds);
    assert_eq!(cmds.commands.len(), cmds.results.len());
    for (cmd, result) in cmds.commands.iter().zip(cmds.results.iter()) {
        let cmd: &Command = cmd;
        match cmd {
            &Command::Malloc { addr, len } => {
                assert_eq!(wmemcheck_state.malloc(addr, len), *result);
            }
            &Command::Free { addr } => {
                assert_eq!(wmemcheck_state.free(addr), *result);
            }
            &Command::Read { addr, len } => {
                assert_eq!(wmemcheck_state.read(addr, len), *result);
            }
            &Command::Write { addr, len } => {
                assert_eq!(wmemcheck_state.write(addr, len), *result);
            }
        }
    }
});

#[derive(Debug)]
pub struct Allocation {
    addr: usize,
    len: usize,
    memstate: Vec<MemState>,
}

impl Allocation {
    fn new(addr: usize, len: usize) -> Allocation {
        Allocation { addr: addr, len: len, memstate: vec![MemState::ValidToWrite; len] }
    }
    fn no_overlaps(&self, other: &Allocation) -> bool {
        other.addr + other.len <= self.addr || self.addr + self.len <= other.addr 
    }
    fn is_in_bounds(&self) -> bool {
        TEST_MAX_STACK_SIZE <= self.addr && self.addr + self.len - 1 <= TEST_MAX_ADDR
    }
}

#[derive(Debug)]
pub enum Command {
    Malloc {addr: usize, len: usize},
    Read {addr: usize, len: usize},
    Write {addr: usize, len: usize},
    Free {addr: usize}
}

#[derive(Debug)]
struct BuggyCommandSequence {
    commands: Vec<Command>,
    results: Vec<Result<(), AccessError>>
}

struct BuggyCommandSequenceState {
    allocations: Vec<Allocation>,
}

impl BuggyCommandSequenceState {
    fn new() -> BuggyCommandSequenceState {
        let allocations = Vec::new();
        BuggyCommandSequenceState { allocations }
    }
    fn update(&mut self, cmd: &Command) {
        match cmd {
            &Command::Malloc { addr, len } => {
                let alloc = Allocation::new(addr, len);
                let validity = is_malloc_valid(&alloc, &self);
                if validity.is_ok() {
                    self.allocations.push(Allocation::new(addr, len));
                }
            }
            &Command::Free { addr } => {
                let validity = is_free_valid(addr, &self);
                if validity.is_ok() {
                    let index = self.allocations.iter().position(|alloc| alloc.addr == addr).unwrap();
                    self.allocations.remove(index);
                }
            }
            &Command::Write { addr, len } => {
                let validity = is_write_valid(addr, len, &self);
                if validity.is_ok() {
                    let index = self.allocations.iter().position(|alloc| alloc.addr <= addr && addr + len <= alloc.addr + alloc.len).unwrap();
                    let alloc_addr = self.allocations[index].addr;
                    let write_to = &mut self.allocations[index].memstate;
                    for i in 0..len {
                        write_to[addr - alloc_addr + i] = MemState::ValidToReadWrite;
                    }
                }
            }
            _ => {}
        }
    }
}


impl<'a> Arbitrary<'a> for BuggyCommandSequence {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<BuggyCommandSequence, libfuzzer_sys::arbitrary::Error> {
        let mut commands = vec![];
        let mut results = vec![];
        let mut state = BuggyCommandSequenceState::new();
        for _ in 0..u.int_in_range(1..=20)? {
            let cmd = match u.int_in_range(0..=3)? {
                0 => {
                    let malloc_addr = u.int_in_range(1..=TEST_MAX_ADDR)?;
                    let malloc_len = u.int_in_range(1..=TEST_MAX_ADDR)?;
                    let alloc = Allocation::new(malloc_addr, malloc_len);
                    results.push(is_malloc_valid(&alloc, &state));
                    Command::Malloc { addr: malloc_addr, len: malloc_len }
                }
                1 => {
                    let choose_rand_addr = u.ratio(1, 2)?;
                    let mut unalloc_addr = 0;
                    if choose_rand_addr {
                        unalloc_addr = u.choose_index(TEST_MAX_ADDR)?;
                    } else {
                        let some_alloc = u.choose_index(state.allocations.len())?;
                        unalloc_addr = state.allocations[some_alloc].addr;
                    }
                    results.push(is_free_valid(unalloc_addr, &state));
                    Command::Free { addr: unalloc_addr }
                }
                2 => {
                    let read_addr = u.choose_index(TEST_MAX_ADDR)?;
                    let read_len = u.int_in_range(1..=TEST_MAX_ADDR)?;
                    results.push(is_read_valid(read_addr, read_len, &state));
                    Command::Read { addr: read_addr, len: read_len }
                }
                3 => {
                    let write_addr = u.choose_index(TEST_MAX_ADDR)?;
                    let write_len = u.int_in_range(1..=TEST_MAX_ADDR)?;
                    results.push(is_write_valid(write_addr, write_len, &state));
                    Command::Write { addr: write_addr, len: write_len }
                }
                _ => {
                    unreachable!()
                }
            };
            // println!("{:?} allocs: {:?} resutls: {:?}", cmd, state.allocations, results);
            state.update(&cmd);
            commands.push(cmd);
        }
        Ok(BuggyCommandSequence { commands, results })
    }
}

fn no_allocs_in_range(state: &BuggyCommandSequenceState, other: &Allocation ) -> bool {
    state.allocations.iter().all(|alloc| alloc.no_overlaps(other))
}

fn is_malloc_valid(alloc: &Allocation, state: &BuggyCommandSequenceState) -> Result<(), AccessError> {
    if !alloc.is_in_bounds() {
        return Err(AccessError::OutOfBounds { addr: alloc.addr, len: alloc.len });
    } else if !no_allocs_in_range(&state, &alloc) {
        return Err(AccessError::DoubleMalloc { addr: alloc.addr, len: alloc.len });
    } else {
        return Ok(());
    }
}

fn is_free_valid(addr: usize, state: &BuggyCommandSequenceState) -> Result<(), AccessError> {
    if !state.allocations.iter().any(|alloc| alloc.addr == addr) {
        return Err(AccessError::InvalidFree { addr });
    } else { 
        return Ok(());
    }
}

fn is_read_valid(addr: usize, len: usize, state: &BuggyCommandSequenceState) -> Result<(), AccessError> {
    let dummy = Allocation::new(addr, len);
    if !dummy.is_in_bounds() {
        return Err(AccessError::OutOfBounds { addr, len });
    }
    let in_range: Vec<_> = state.allocations.iter()
                                    .filter(|alloc| alloc.addr <= addr && 
                                        addr + len <= alloc.addr + alloc.len && alloc.memstate.contains(&MemState::ValidToReadWrite)).collect();
    if in_range.is_empty() {
        return Err(AccessError::InvalidRead { addr, len });
    } else {
        let memstate_addr = addr - &in_range[0].addr;
        for i in memstate_addr..memstate_addr + len {
            // println!("{:?}", mem_index);
            if in_range[0].memstate[i] != MemState::ValidToReadWrite {
                return Err(AccessError::InvalidRead { addr, len });
            }
        }
        return Ok(());
    }
}

fn is_write_valid(addr: usize, len: usize, state: &BuggyCommandSequenceState) -> Result<(), AccessError> {
    let dummy = Allocation::new(addr, len);
    //this doesn't include stack... have to change to include validity for stack read/writes
    if !dummy.is_in_bounds() {
        return Err(AccessError::OutOfBounds { addr, len });
    }
    if !state.allocations.iter().any(|alloc| alloc.addr <= addr && addr + len <= alloc.addr + alloc.len) {
        return Err(AccessError::InvalidWrite { addr, len });
    } else { 
        return Ok(());
    }
}