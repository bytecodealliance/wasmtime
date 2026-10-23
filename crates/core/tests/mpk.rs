//! Checks that `mpk::is_supported` agrees with what the hardware and kernel
//! actually allow.
//!
//! We test that `RDPKRU`/`WRPKRU` work if and only if `mpk::is_supported` returns true.
//! The => direction is required to avoid hard to debug SIGILLs if our is_supported logic was wrong.
//! The other direction allows us to detect if we're missing out on using MPKs in places they are
//! actually allowed.
//!
//! This is still linux and x86 specific.
#![cfg(all(target_os = "linux", target_arch = "x86_64", not(miri)))]

use std::arch::asm;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, exit};

/// Set in the child process to make it run the probe instead of the test.
const PROBE_ENV_VAR: &str = "WASMTIME_TEST_MPK_PROBE";

/// Name of the test below, used to run only it in the child process.
const TEST_NAME: &str = "is_supported_matches_hardware";

// Exit codes of the probe, besides success.
const EXIT_PKEY_ALLOC_FAILED: i32 = 2;
const EXIT_PKEY_FREE_FAILED: i32 = 3;

#[test]
fn is_supported_matches_hardware() {
    // We are the child that is supposed to perform the probe.
    if std::env::var_os(PROBE_ENV_VAR).is_some() {
        probe();
        // probe always exits, this is unreachable
    }

    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
        .env(PROBE_ENV_VAR, "1")
        .output()
        .expect("failed to spawn the probe process");
    let usable = if output.status.success() {
        true
    } else if output.status.signal() == Some(libc::SIGILL) {
        false
    } else {
        panic!(
            "inconclusive probe for protection key support: {}\n\
             stdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    };

    assert_eq!(
        wasmtime_internal_core::mpk::is_supported(),
        usable,
        "`mpk::is_supported()` disagrees with whether protection keys can \
         actually be used on this host",
    );
}

/// Checks `RDPKRU`, `WRPKRU`, and tries to allocate and free a pkey.
/// Exits the process with 0 if everything worked, raises `SIGILL` otherwise.
fn probe() -> ! {
    // No crate offers rdpkru or wrpkru as a Rust function (neither rustix nor libc), so we need to
    // rawdog asm.
    let pkru: u32;
    // SAFETY: reading PKRU has no side effects; if protection keys are not
    // enabled this raises `SIGILL`, which is what the parent checks for.
    unsafe {
        asm!("rdpkru", in("ecx") 0u32, out("eax") pkru, out("edx") _,
            options(nomem, nostack, preserves_flags));
    }
    // SAFETY: writing back the same value doesn't change any access rights.
    unsafe {
        asm!("wrpkru", in("eax") pkru, in("ecx") 0u32, in("edx") 0u32,
            options(nomem, nostack, preserves_flags));
    }

    // SAFETY: `pkey_alloc(flags = 0, access_rights = 0)` only allocates a key.
    let key = unsafe { libc::syscall(libc::SYS_pkey_alloc, 0, 0) };
    if key < 0 {
        eprintln!("pkey_alloc failed: {}", io::Error::last_os_error());
        exit(EXIT_PKEY_ALLOC_FAILED);
    }
    // SAFETY: frees the key just allocated, which nothing uses.
    let ret = unsafe { libc::syscall(libc::SYS_pkey_free, key as usize, 0) };
    if ret != 0 {
        eprintln!("pkey_free failed: {}", io::Error::last_os_error());
        exit(EXIT_PKEY_FREE_FAILED);
    }
    exit(0)
}
