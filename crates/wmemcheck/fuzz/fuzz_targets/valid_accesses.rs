#![no_main]

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured, Error};
use wmemcheck::{Wmemcheck, MemState};
use std::cmp::*;

const TEST_MAX_ADDR: usize = 1024 * 640 - 1;
const TEST_MAX_STACK_SIZE: usize = 1024;

fuzz_target!(|data: &[u8]| {
    let u = &mut Unstructured::new(data);
    let mut wmemcheck_state = Wmemcheck::new(TEST_MAX_ADDR + 1, TEST_MAX_STACK_SIZE);
    let cmds = match CommandSequence::arbitrary(u) {
        Ok(val) => val,
        Err(_) => return,
    };
    println!("commands: {:?}", cmds);
    for cmd in cmds.commands.iter() {
        let cmd: &Command = cmd;
        match cmd {
            &Command::Malloc { addr, len } => {
                assert!(wmemcheck_state.malloc(addr, len).is_ok());
            }
            &Command::Free { addr } => {
                assert!(wmemcheck_state.free(addr).is_ok());
            }
            &Command::Read { addr, len } => {
                assert!(wmemcheck_state.read(addr, len).is_ok());
            }
            &Command::Write { addr, len } => {
                assert!(wmemcheck_state.write(addr, len).is_ok());
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
enum Command {
    Malloc {addr: usize, len: usize},
    Read {addr: usize, len: usize},
    Write {addr: usize, len: usize},
    Free {addr: usize}
}

#[derive(Debug)]
struct CommandSequence {
    commands: Vec<Command>,
}

struct CommandSequenceState {
    allocations: Vec<Allocation>,
}

impl CommandSequenceState {
    fn new() -> CommandSequenceState {
        let allocations = Vec::new();
        CommandSequenceState { allocations }
    }
    fn update(&mut self, cmd: &Command) {
        match cmd {
            &Command::Malloc { addr, len } => {
                self.allocations.push(Allocation::new(addr, len)); 
            }
            &Command::Free { addr } => {
                let index = self.allocations.iter().position(|alloc| alloc.addr == addr).unwrap();
                self.allocations.remove(index);
            }
            &Command::Write { addr, len } => {
                let index = self.allocations.iter().position(|alloc| alloc.addr <= addr && addr + len <= alloc.addr + alloc.len).unwrap();
                let alloc_addr = self.allocations[index].addr;
                let write_to = &mut self.allocations[index].memstate;
                for i in 0..len {
                    write_to[addr - alloc_addr + i] = MemState::ValidToReadWrite;
                }
            }
            _ => {}
        }
    }
 }

impl<'a> Arbitrary<'a> for CommandSequence {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<CommandSequence, libfuzzer_sys::arbitrary::Error> {
        let mut commands = vec![];
        let mut state = CommandSequenceState::new();
        for _ in 0..u.arbitrary::<usize>()? {
            let cmd = match u.int_in_range(0..=3)? {
                0 => {
                    let malloc_range = pick_free_addr_range(&state, u)?;
                    Command::Malloc { addr: malloc_range.0, len: malloc_range.1 }
                }
                1 => {
                    let unalloc_index = u.choose_index(state.allocations.len())?;
                    let unalloc_addr = state.allocations[unalloc_index].addr;
                    Command::Free { addr: unalloc_addr }
                }
                2 => {
                    let read_range = pick_read_range(&state, u)?;
                    Command::Read { addr: read_range.0, len: read_range.1 }
                }
                3 => {
                    let write_index = u.choose_index(state.allocations.len())?;
                    let mut write_range = 1;
                    if state.allocations[write_index].len > 1 {
                        write_range = u.int_in_range(1..=state.allocations[write_index].len)?;
                    }
                    Command::Write { addr: state.allocations[write_index].addr, len: write_range }
                }
                _ => {
                    unreachable!()
                }
            };
            println!("{:?}", cmd);
            state.update(&cmd);
            commands.push(cmd);
        }
        Ok(CommandSequence { commands })
    }
}

fn pick_free_addr_range(state: &CommandSequenceState, u: &mut Unstructured<'_>) -> Result<(usize, usize), Error> {
    let mut addr = u.int_in_range(TEST_MAX_STACK_SIZE..=TEST_MAX_ADDR)?;
    let dummy_alloc = Allocation::new(addr, 1);
    let mut attempts = 0;
    while !no_allocs_in_range(state, &dummy_alloc) {
        addr = u.int_in_range(1024..=TEST_MAX_ADDR)?;
        attempts += 1;
        if attempts == 10 {
            return Err(Error::NotEnoughData);
        }
    }
    let mut len = 1;
    if TEST_MAX_ADDR - addr > 1 {
        len = u.int_in_range(1..=TEST_MAX_ADDR - addr)?;
    }
    attempts = 0;
    while !no_allocs_in_range(state, &Allocation::new(addr, len)) {
        if TEST_MAX_ADDR - addr > 1 {
            len = u.int_in_range(1..=TEST_MAX_ADDR - addr)?;
        }
        attempts += 1;
        if attempts == 10 {
            return Err(Error::NotEnoughData);
        }
    }
    Ok((addr, len))
}

fn pick_read_range(state: &CommandSequenceState, u: &mut Unstructured<'_>) -> Result<(usize, usize), Error> {
    if state.allocations.is_empty() {
        return Err(Error::NotEnoughData);
    }
    let mut alloc_index = u.choose_index(state.allocations.len())?;
    let mut attempts = 0;
    while !state.allocations[alloc_index].memstate.contains(&MemState::ValidToReadWrite) {
        alloc_index = u.choose_index(state.allocations.len())?;
        attempts += 1;
        if attempts == min(state.allocations.len(), 10) {
            return Err(Error::NotEnoughData);
        }
    }
    let mut memstate_addr = u.int_in_range(0..=state.allocations[alloc_index].len - 1)?;
    attempts = 0;
    while let MemState::ValidToWrite = state.allocations[alloc_index].memstate[memstate_addr] {
        memstate_addr = u.int_in_range(0..=state.allocations[alloc_index].len - 1)?;
        attempts += 1;
        if attempts == 10 {
            return Err(Error::NotEnoughData);
        }
    }
    if state.allocations[alloc_index].memstate.len() - memstate_addr <= 1 {
        return Ok((state.allocations[alloc_index].addr + memstate_addr, 1));
    }
    let mut len = u.int_in_range(0..=state.allocations[alloc_index].memstate.len() - memstate_addr)?;
    attempts = 0;
    while !ok_range(state, alloc_index, memstate_addr, memstate_addr + len) {
        len = u.int_in_range(0..=state.allocations[alloc_index].memstate.len() - 1)?;
        attempts += 1;
        if attempts == 10 {
            return Err(Error::NotEnoughData);
        }
    }
    Ok((state.allocations[alloc_index].addr + memstate_addr, len))
}

fn ok_range(state: &CommandSequenceState, alloc_index: usize, start: usize, end: usize) -> bool {
    for i in start..end {
        if let MemState::ValidToWrite = state.allocations[alloc_index].memstate[i] {
            return false;
        }
    }
    true
}

fn no_allocs_in_range(state: &CommandSequenceState, other: &Allocation ) -> bool {
    state.allocations.iter().all(|alloc| alloc.no_overlaps(other))
}