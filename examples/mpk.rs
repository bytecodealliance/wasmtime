//! This example demonstrates:
//! - how to enable memory protection keys (MPK) in a Wasmtime embedding (see
//!   [`build_engine`])
//! - the expected memory compression from using MPK: it will probe the system
//!   by creating larger and larger memory pools until system memory is
//!   exhausted (see [`probe_engine_size`]). Then, it prints a comparison of the
//!   memory used in both the MPK enabled and MPK disabled configurations.
//!
//! You can execute this example with:
//!
//! ```console
//! $ cargo run --example mpk
//! ```
//!
//! Append `-- --help` for details about the configuring the memory size of the
//! pool. Also, to inspect interesting configuration values used for
//! constructing the pool, turn on logging:
//!
//! ```console
//! $ RUST_LOG=debug cargo run --example mpk -- --memory-size 512MiB
//! ```
//!
//! Note that MPK support is limited to x86 Linux systems. OS limits on the
//! number of virtual memory areas (VMAs) can significantly restrict the total
//! number MPK-striped memory slots; each MPK-protected slot ends up using a new
//! VMA entry. On Linux, one can raise this limit:
//!
//! ```console
//! $ sysctl vm.max_map_count
//! 65530
//! $ sysctl vm.max_map_count=$LARGER_LIMIT
//! ```

use anyhow::anyhow;
use bytesize::ByteSize;
use clap::Parser;
use log::{info, warn};
use std::str::FromStr;
use wasmtime::*;

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    info!("{:?}", args);

    let without_mpk = probe_engine_size(&args, MpkEnabled::Disable)?;
    println!("without MPK:\t{}", without_mpk.to_string());

    if PoolingAllocationConfig::are_memory_protection_keys_available() {
        let with_mpk = probe_engine_size(&args, MpkEnabled::Enable)?;
        println!("with MPK:\t{}", with_mpk.to_string());
        println!(
            "\t\t{}x more slots per reserved memory",
            with_mpk.compare(&without_mpk)
        );
    } else {
        println!("with MPK:\tunavailable\t\tunavailable");
    }

    Ok(())
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The maximum number of bytes for each WebAssembly linear memory in the
    /// pool.
    #[arg(long, default_value = "128MiB", value_parser = parse_byte_size)]
    memory_size: usize,

    /// The maximum number of bytes a memory is considered static; see
    /// `Config::static_memory_maximum_size` for more details and the default
    /// value if unset.
    #[arg(long, value_parser = parse_byte_size)]
    static_memory_maximum_size: Option<u64>,

    /// The size in bytes of the guard region to expect between static memory
    /// slots; see [`Config::static_memory_guard_size`] for more details and the
    /// default value if unset.
    #[arg(long, value_parser = parse_byte_size)]
    static_memory_guard_size: Option<u64>,
}

/// Parse a human-readable byte size--e.g., "512 MiB"--into the correct number
/// of bytes.
fn parse_byte_size(value: &str) -> Result<u64> {
    let size = ByteSize::from_str(value).map_err(|e| anyhow!(e))?;
    Ok(size.as_u64())
}

/// Find the engine with the largest number of memories we can create on this
/// machine.
fn probe_engine_size(args: &Args, mpk: MpkEnabled) -> Result<Pool> {
    let mut search = ExponentialSearch::new();
    let mut mapped_bytes = 0;
    while !search.done() {
        match build_engine(&args, search.next(), mpk) {
            Ok(rb) => {
                // TODO: assert!(rb >= mapped_bytes);
                mapped_bytes = rb;
                search.record(true)
            }
            Err(e) => {
                warn!("failed engine allocation, continuing search: {:?}", e);
                search.record(false)
            }
        }
    }
    Ok(Pool {
        num_memories: search.next(),
        mapped_bytes,
    })
}

#[derive(Debug)]
#[allow(dead_code)]
struct Pool {
    num_memories: u32,
    mapped_bytes: usize,
}
impl Pool {
    /// Print a human-readable, tab-separated description of this structure.
    fn to_string(&self) -> String {
        let human_size = ByteSize::b(self.mapped_bytes as u64).to_string_as(true);
        format!(
            "{} memory slots\t{} reserved",
            self.num_memories, human_size
        )
    }
    /// Return the number of times more memory slots in `self` than `other`
    /// after normalizing by the mapped bytes sizes. Rounds to three decimal
    /// places arbitrarily; no significance intended.
    fn compare(&self, other: &Pool) -> f64 {
        let size_ratio = other.mapped_bytes as f64 / self.mapped_bytes as f64;
        let slots_ratio = self.num_memories as f64 / other.num_memories as f64;
        let times_more_efficient = slots_ratio * size_ratio;
        (times_more_efficient * 1000.0).round() / 1000.0
    }
}

/// Exponentially increase the `next` value until the attempts fail, then
/// perform a binary search to find the maximum attempted value that still
/// succeeds.
#[derive(Debug)]
struct ExponentialSearch {
    /// Determines if we are in the growth phase.
    growing: bool,
    /// The last successful value tried; this is the algorithm's lower bound.
    last: u32,
    /// The next value to try; this is the algorithm's upper bound.
    next: u32,
}
impl ExponentialSearch {
    fn new() -> Self {
        Self {
            growing: true,
            last: 0,
            next: 1,
        }
    }
    fn next(&self) -> u32 {
        self.next
    }
    fn record(&mut self, success: bool) {
        if !success {
            self.growing = false
        }
        let diff = if self.growing {
            (self.next - self.last) * 2
        } else {
            (self.next - self.last + 1) / 2
        };
        if success {
            self.last = self.next;
            self.next = self.next + diff;
        } else {
            self.next = self.next - diff;
        }
    }
    fn done(&self) -> bool {
        self.last == self.next
    }
}

/// Build a pool-allocated engine with `num_memories` slots.
fn build_engine(args: &Args, num_memories: u32, enable_mpk: MpkEnabled) -> Result<usize> {
    // Configure the memory pool.
    let mut pool = PoolingAllocationConfig::default();
    pool.max_memory_size(args.memory_size);
    pool.total_memories(num_memories)
        .memory_protection_keys(enable_mpk);

    // Configure the engine itself.
    let mut config = Config::new();
    if let Some(static_memory_maximum_size) = args.static_memory_maximum_size {
        config.static_memory_maximum_size(static_memory_maximum_size);
    }
    if let Some(static_memory_guard_size) = args.static_memory_guard_size {
        config.static_memory_guard_size(static_memory_guard_size);
    }
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    // Measure memory use before and after the engine is built.
    let mapped_bytes_before = num_bytes_mapped()?;
    let engine = Engine::new(&config)?;
    let mapped_bytes_after = num_bytes_mapped()?;

    // Ensure we actually use the engine somehow.
    engine.increment_epoch();

    let mapped_bytes = mapped_bytes_after - mapped_bytes_before;
    info!(
        "{}-slot pool ({:?}): {} bytes mapped",
        num_memories, enable_mpk, mapped_bytes
    );
    Ok(mapped_bytes)
}

/// Add up the sizes of all the mapped virtual memory regions for the current
/// Linux process.
///
/// This manually parses `/proc/self/maps` to avoid a rather-large `proc-maps`
/// dependency. We do expect this example to be Linux-specific anyways. For
/// reference, lines of that file look like:
///
/// ```text
/// 5652d4418000-5652d441a000 r--p 00000000 00:23 84629427 /usr/bin/...
/// ```
///
/// We parse the start and end addresses: <start>-<end> [ignore the rest].
#[cfg(target_os = "linux")]
fn num_bytes_mapped() -> Result<usize> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open("/proc/self/maps")?;
    let reader = BufReader::new(file);
    let mut total = 0;
    for line in reader.lines() {
        let line = line?;
        let range = line
            .split_whitespace()
            .next()
            .ok_or(anyhow!("parse failure: expected whitespace"))?;
        let mut addresses = range.split("-");
        let start = addresses
            .next()
            .ok_or(anyhow!("parse failure: expected dash-separated address"))?;
        let start = usize::from_str_radix(start, 16)?;
        let end = addresses
            .next()
            .ok_or(anyhow!("parse failure: expected dash-separated address"))?;
        let end = usize::from_str_radix(end, 16)?;

        total += end - start;
    }
    Ok(total)
}

#[cfg(not(target_os = "linux"))]
fn num_bytes_mapped() -> Result<usize> {
    anyhow::bail!("this example can only read virtual memory maps on Linux")
}
