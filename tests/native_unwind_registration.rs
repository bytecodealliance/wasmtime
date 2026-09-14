//! This test needs its own executable because it queries unwind info after
//! dropping modules. Another test in the same process could reuse a dropped
//! module's code addresses and make the removal check find the new module's
//! unwind info instead.

#![cfg(all(
    any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")),
    not(miri)
))]

use wasmtime::{Config, Engine, Module, Result};

#[repr(C)]
#[derive(Default)]
struct DwarfEhBases {
    tbase: usize,
    dbase: usize,
    func: usize,
}

unsafe extern "C" {
    fn _Unwind_Find_FDE(pc: *const u8, bases: *mut DwarfEhBases) -> *const u8;
}

fn find_fde(pc: usize) -> (usize, usize) {
    let mut bases = DwarfEhBases::default();
    // SAFETY: the API only looks up the address, and bases is a valid out pointer.
    let fde = unsafe { _Unwind_Find_FDE(pc as *const u8, &mut bases) as usize };
    (fde, bases.func)
}

fn check_module(module: &Module) -> Vec<usize> {
    let start = module.text().as_ptr() as usize;
    let image = module.image_range();
    let pcs = module
        .functions()
        .map(|f| start + f.offset + 1)
        .collect::<Vec<_>>();
    assert_eq!(pcs.len(), 3, "expected all three defined functions");
    for pc in &pcs {
        let (fde, func) = find_fde(*pc);
        assert!(fde >= image.start as usize && fde < image.end as usize);
        assert_eq!(func, pc - 1);
    }

    pcs
}

#[test]
fn native_unwind_sections_survive_non_lifo_removal() -> Result<()> {
    let mut config = Config::new();
    config.native_unwind_info(true);
    let engine = Engine::new(&config)?;
    let wasm = wat::parse_str(
        r#"(module
            (import "" "capture" (func $capture))
            (func $inner call $capture)
            (func $middle call $inner)
            (func (export "run") call $middle))"#,
    )?;
    let make_module = || Module::from_binary(&engine, &wasm);
    let first = make_module()?;
    let middle = make_module()?;
    let last = make_module()?;
    check_module(&first);
    let middle_pcs = check_module(&middle);
    check_module(&last);

    drop(middle);
    for pc in middle_pcs {
        assert_eq!(
            find_fde(pc).0,
            0,
            "dropped module still has registered unwind info"
        );
    }
    let first_pcs = check_module(&first);
    check_module(&last);
    drop(first);
    for pc in first_pcs {
        assert_eq!(find_fde(pc).0, 0);
    }
    let last_pcs = check_module(&last);
    drop(last);
    for pc in last_pcs {
        assert_eq!(find_fde(pc).0, 0);
    }
    // Exercise registration again after the registry has removed every section.
    let fresh = make_module()?;
    let fresh_pcs = check_module(&fresh);
    drop(fresh);
    for pc in fresh_pcs {
        assert_eq!(find_fde(pc).0, 0);
    }
    Ok(())
}
