//! Checks that `mpk::is_supported` agrees with what the hardware and kernel
//! actually allow.
//!
//! We test that `RDPKRU`/`WRPKRU` work only if `mpk::is_supported` returns true.
//! This is required to avoid hard to debug SIGILLs if our is_supported logic was wrong.
//!
//! On unsupported platforms we just test that is_supported is false.
//! There are platforms where MPK is supported by hardware and the OS (32-bit x86, FreeBSD, ARM targets),
//! but Wasmtime has no backend for that.

use libtest_mimic::{Arguments, Failed, Trial};
use std::process::ExitStatus;
use std::process::{exit, Command};

/// Set in the child process to make it run the probe instead of the test.
const PROBE_ENV_VAR: &str = "WASMTIME_TEST_MPK_PROBE";

// Exit codes of the probe, besides success.
pub const EXIT_PKEY_ALLOC_FAILED: i32 = 2;
pub const EXIT_PKEY_FREE_FAILED: i32 = 3;
pub const EXIT_MPK_NOT_SUPPORTED_BY_WASMTIME: i32 = 4;

fn main() {
    // We are the child that is supposed to perform the probe.
    if std::env::var_os(PROBE_ENV_VAR).is_some() {
        probe();
        // probe always exits, this is unreachable
    }
    // We are the test entrypoint.
    let run_test = || {
        let output = Command::new(std::env::current_exe().unwrap())
            .env(PROBE_ENV_VAR, "1")
            .output()
            .expect("failed to spawn the probe process");
        let usable = if output.status.success() {
            true
        } else if is_illegal_instruction(&output.status)
            || is_not_supported_by_wasmtime(&output.status)
        {
            false
        } else {
            return Err(Failed::from(format!(
                "inconclusive probe for protection key support: {}\n\
                 stdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            )));
        };

        let is_supported = wasmtime_internal_core::mpk::is_supported();
        if is_supported != usable {
            Err(Failed::from(format!(
                "`mpk::is_supported()` disagrees with whether protection keys can \
                 actually be used on this host: is_supported() = {is_supported}, usable = {usable}",
            )))
        } else {
            Ok(())
        }
    };
    let trial = Trial::test("is_supported_matches_hardware_and_os", run_test)
        // Has to be excluded on miri as we can't even spawn the subcommand there.
        .with_ignored_flag(cfg!(miri));
    libtest_mimic::run(&Arguments::from_args(), vec![trial]).exit();
}

fn is_not_supported_by_wasmtime(status: &ExitStatus) -> bool {
    status
        .code()
        .is_some_and(|x| x == EXIT_MPK_NOT_SUPPORTED_BY_WASMTIME)
}

fn is_illegal_instruction(status: &ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    status.signal() == Some(libc::SIGILL)
}

/// Checks `RDPKRU`, `WRPKRU`, and tries to allocate and free a pkey.
/// Exits the process with 0 if everything worked, raises `SIGILL` otherwise.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn probe() -> ! {
    use std::arch::asm;
    use std::io;
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
    let ret = unsafe { libc::syscall(libc::SYS_pkey_free, key as usize) };
    if ret != 0 {
        eprintln!("pkey_free failed: {}", io::Error::last_os_error());
        exit(EXIT_PKEY_FREE_FAILED);
    }
    exit(0)
}

/// Some platforms like ARM or FreeBSD support MPKs, but with different APIs that wasmtime
/// currently does not hook into.
/// We just return a special error on those.
/// If wasmtime gets MPK support on those platforms, this should be updated to check the
/// capabilities of the relevant API.
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn probe() -> ! {
    exit(EXIT_MPK_NOT_SUPPORTED_BY_WASMTIME)
}
