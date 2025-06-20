use anyhow::{Result, bail, format_err};
use filecheck::{CheckerBuilder, NO_VARIABLES};
use std::env;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;
use test_programs_artifacts::*;

macro_rules! assert_test_exists {
    ($name:ident) => {
        #[expect(unused_imports, reason = "here to assert tests exist")]
        use self::$name as _;
    };
}
foreach_dwarf!(assert_test_exists);

fn lldb_with_script(args: &[&str], script: &str) -> Result<String> {
    let _ = env_logger::try_init();

    let lldb_path = env::var("LLDB").unwrap_or("lldb".to_string());
    let mut cmd = Command::new(&lldb_path);

    cmd.arg("--batch");
    if cfg!(target_os = "macos") {
        cmd.args(&["-o", "settings set plugin.jit-loader.gdb.enable on"]);
    }
    let mut script_file = NamedTempFile::new()?;
    script_file.write(script.as_bytes())?;
    let script_path = script_file.path().to_str().unwrap();
    cmd.args(&["-s", &script_path]);

    let mut me = std::env::current_exe().expect("current_exe specified");
    me.pop(); // chop off the file name
    me.pop(); // chop off `deps`
    if cfg!(target_os = "windows") {
        me.push("wasmtime.exe");
    } else {
        me.push("wasmtime");
    }
    cmd.arg(me);

    cmd.arg("--");
    cmd.args(args);

    log::trace!("Running command: {cmd:?}");
    let output = cmd.output().expect("success");

    let stdout = String::from_utf8(output.stdout)?;
    log::trace!("--- sdout ---\n{stdout}");

    let stderr = String::from_utf8(output.stderr)?;
    log::trace!("--- sderr ---\n{stderr}");

    if !output.status.success() {
        bail!(
            "failed to execute {cmd:?}:\n\
            --- stderr ---\n\
            {stderr}\n\
            --- stdout ---\n\
            {stdout}",
        );
    }
    Ok(stdout)
}

fn check_lldb_output(output: &str, directives: &str) -> Result<()> {
    let mut builder = CheckerBuilder::new();
    builder
        .text(directives)
        .map_err(|e| format_err!("unable to build checker: {:?}", e))?;
    let checker = builder.finish();
    let check = checker
        .explain(output, NO_VARIABLES)
        .map_err(|e| format_err!("{:?}", e))?;
    assert!(check.0, "didn't pass check {}", check.1);
    Ok(())
}

#[allow(dead_code, reason = "tested elsewhere")]
fn dwarf_dead_code() {} // this is tested over in `translate.rs`

#[test]
#[ignore]
pub fn dwarf_fib_wasm() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Ddebug-info",
            "-Oopt-level=0",
            "--invoke",
            "fib",
            DWARF_FIB_WASM,
            "3",
        ],
        r#"b fib
r
fr v
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: Breakpoint 1: no locations (pending)
check: Unable to resolve breakpoint to any actual locations.
check: 1 location added to breakpoint 1
check: stop reason = breakpoint 1.1
check: frame #0
sameln: JIT
sameln: fib(n=3)
check: n = 3
check: a = 0
check: resuming
check: exited with status
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
pub fn dwarf_fib_wasm_dwarf5() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Ddebug-info",
            "-Oopt-level=0",
            "--invoke",
            "fib",
            DWARF_FIB_WASM_DWARF5,
            "3",
        ],
        r#"b fib
r
fr v
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: Breakpoint 1: no locations (pending)
check: Unable to resolve breakpoint to any actual locations.
check: 1 location added to breakpoint 1
check: stop reason = breakpoint 1.1
check: frame #0
sameln: JIT
sameln: fib(n=3)
check: n = 3
check: a = 0
check: resuming
check: exited with status
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
pub fn dwarf_fib_wasm_split4() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Ddebug-info",
            "-Oopt-level=0",
            "--invoke",
            "fib",
            DWARF_FIB_WASM_SPLIT4,
            "3",
        ],
        r#"b fib
r
fr v
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: Breakpoint 1: no locations (pending)
check: Unable to resolve breakpoint to any actual locations.
check: 1 location added to breakpoint 1
check: stop reason = breakpoint 1.1
check: frame #0
sameln: JIT
sameln: fib(n=3)
check: n = 3
check: a = 0
check: resuming
check: exited with status
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
pub fn dwarf_generic() -> Result<()> {
    let output = lldb_with_script(
        &["-Ccache=n", "-Ddebug-info", "-Oopt-level=0", DWARF_GENERIC],
        r#"br set -n debug_break -C up
r
p __vmctx->set()
p (x + x)
c
p (x + x)
c
p inst.BaseValue + inst.DerivedValue
c
type lookup DerivedType
c
p __this->BaseValue + __this->DerivedValue
c
p __this->BaseValue + __this->DerivedValue
c
p __this->BaseValue + __this->DerivedValue
c
f
n
s
v var0
v var1
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: stop reason = breakpoint 1.1
check: 2
check: stop reason = breakpoint 1.1
check: 4
check: stop reason = breakpoint 1.1
check: 3
check: stop reason = breakpoint 1.1
check: static int InstanceMethod
check: static int ConstInstanceMethod
check: stop reason = breakpoint 1.1
check: 6
check: stop reason = breakpoint 1.1
check: 7
check: stop reason = breakpoint 1.1
check: 8
check: stop reason = breakpoint 1.1
check: 9
check: 10
check: exited with status = 0
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
pub fn dwarf_codegen_optimized() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Ddebug-info",
            "-Oopt-level=2",
            DWARF_CODEGEN_OPTIMIZED,
        ],
        r#"b InitializeTest
r
b dwarf_codegen_optimized.cpp:25
b dwarf_codegen_optimized.cpp:26
c
v x
c
v x
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: stop reason = breakpoint 1.1
check: stop reason = breakpoint 2.1
check: x = 42
check: stop reason = breakpoint 3.1
check: x = <variable not available>
check: exited with status = 0
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
pub fn dwarf_codegen_optimized_wasm_optimized() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Ddebug-info",
            "-Oopt-level=2",
            DWARF_CODEGEN_OPTIMIZED_WASM_OPTIMIZED,
        ],
        r#"b InitializeTest
r
b dwarf_codegen_optimized_wasm_optimized.cpp:23
b dwarf_codegen_optimized_wasm_optimized.cpp:29
c
v b
c
v b
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: stop reason = breakpoint 1.1
check: stop reason = breakpoint 2.1
check: b = 42
check: stop reason = breakpoint 3.1
check: b = 43
check: exited with status = 0
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
pub fn dwarf_fraction_norm() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Oopt-level=0",
            "-Ddebug-info",
            DWARF_FRACTION_NORM,
        ],
        r#"b dwarf_fraction_norm.cc:26
r
p __vmctx->set(),n->denominator
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: Breakpoint 1: no locations (pending)
check: stop reason = breakpoint 1.1
check: frame #0
sameln: norm(n=(__ptr =
check: 27
check: resuming
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
pub fn dwarf_two_removed_branches() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Oopt-level=0",
            "-Ddebug-info",
            DWARF_TWO_REMOVED_BRANCHES,
        ],
        r#"r"#,
    )?;

    // We are simply checking that the output compiles.
    check_lldb_output(
        &output,
        r#"
check: exited with status
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
pub fn dwarf_spilled_frame_base() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Oopt-level=0",
            "-Ddebug-info",
            DWARF_SPILLED_FRAME_BASE,
        ],
        r#"b dwarf_spilled_frame_base.c:13
r
fr v i
n
fr v i
n
fr v i
n
fr v i
n
fr v i
n
fr v i
c
"#,
    )?;

    // Check that if the frame base (shadow frame pointer) local
    // is spilled, we can still read locals that reference it.
    check_lldb_output(
        &output,
        r#"
check: i = 0
check: i = 1
check: i = 1
check: i = 1
check: i = 1
check: i = 1
check: exited with status
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
pub fn dwarf_fission() -> Result<()> {
    let output = lldb_with_script(
        &["-Ccache=n", "-Ddebug-info", "-Oopt-level=0", DWARF_FISSION],
        r#"breakpoint set --file dwarf_fission.c --line 8
r
fr v
s
fr v
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: Breakpoint 1: no locations (pending)
check: Unable to resolve breakpoint to any actual locations.
check: 1 location added to breakpoint 1
check: stop reason = breakpoint 1.1
check: i = 1
check: stop reason = step in
check: i = 2
check: resuming
check: exited with status = 0
"#,
    )?;
    Ok(())
}

fn test_dwarf_simple(wasm: &str, extra_args: &[&str]) -> Result<()> {
    println!("testing {wasm:?}");
    let mut args = vec!["-Ccache=n", "-Oopt-level=0", "-Ddebug-info"];
    args.extend(extra_args);
    args.push(wasm);
    let output = lldb_with_script(
        &args,
        r#"
breakpoint set --file dwarf_simple.rs --line 3
breakpoint set --file dwarf_simple.rs --line 5
r
fr v
c
fr v
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: Breakpoint 1: no locations (pending)
check: Unable to resolve breakpoint to any actual locations.
check: 1 location added to breakpoint 1
check: stop reason = breakpoint 1.1
check: dwarf_simple.rs:3
check: a = 100
check: dwarf_simple.rs:5
check: a = 110
check: b = 117
check: resuming
check: exited with status = 0
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
fn dwarf_simple() -> Result<()> {
    for wasm in [DWARF_SIMPLE, DWARF_SIMPLE_COMPONENT] {
        test_dwarf_simple(wasm, &[])?;
    }
    Ok(())
}

#[test]
#[ignore]
fn dwarf_imported_memory() -> Result<()> {
    test_dwarf_simple(
        DWARF_IMPORTED_MEMORY,
        &["--preload=env=./tests/all/debug/satisfy_memory_import.wat"],
    )
}

#[test]
#[ignore]
fn dwarf_shared_memory() -> Result<()> {
    test_dwarf_simple(DWARF_SHARED_MEMORY, &[])
}

#[test]
#[ignore]
fn dwarf_multiple_codegen_units() -> Result<()> {
    for wasm in [
        DWARF_MULTIPLE_CODEGEN_UNITS,
        DWARF_MULTIPLE_CODEGEN_UNITS_COMPONENT,
    ] {
        println!("testing {wasm:?}");
        let output = lldb_with_script(
            &["-Ccache=n", "-Oopt-level=0", "-Ddebug-info", wasm],
            r#"
breakpoint set --file dwarf_multiple_codegen_units.rs --line 3
breakpoint set --file dwarf_multiple_codegen_units.rs --line 10
r
fr v
c
fr v
breakpoint delete 2
finish
c"#,
        )?;

        check_lldb_output(
            &output,
            r#"
check: Breakpoint 1: no locations (pending)
check: Breakpoint 2: no locations (pending)
check: stop reason = breakpoint 1.1
check: foo::bar(a)
check: a = 3
check: sum += i
check: x = 3
check: sum = 0
check: 1 breakpoints deleted
check: Return value: $(=.*) 3
check: exited with status = 0
"#,
        )?;
    }
    Ok(())
}
