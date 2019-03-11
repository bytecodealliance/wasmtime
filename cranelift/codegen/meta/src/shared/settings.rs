use crate::cdsl::settings::{SettingGroup, SettingGroupBuilder};

pub fn define() -> SettingGroup {
    let mut settings = SettingGroupBuilder::new("shared");

    settings.add_enum(
        "opt_level",
        r#"
        Optimization level:

        - default: Very profitable optimizations enabled, none slow.
        - best: Enable all optimizations
        - fastest: Optimize for compile time by disabling most optimizations.
        "#,
        vec!["default", "best", "fastest"],
    );

    settings.add_bool(
        "enable_verifier",
        r#"
        Run the Cranelift IR verifier at strategic times during compilation.

        This makes compilation slower but catches many bugs. The verifier is
        disabled by default, except when reading Cranelift IR from a text file.
        "#,
        true,
    );

    // Note that Cranelift doesn't currently need an is_pie flag, because PIE is
    // just PIC where symbols can't be pre-empted, which can be expressed with the
    // `colocated` flag on external functions and global values.
    settings.add_bool(
        "is_pic",
        "Enable Position-Independent Code generation",
        false,
    );

    settings.add_bool(
        "colocated_libcalls",
        r#"
            Use colocated libcalls.

            Generate code that assumes that libcalls can be declared "colocated",
            meaning they will be defined along with the current function, such that
            they can use more efficient addressing.
            "#,
        false,
    );

    settings.add_bool(
        "avoid_div_traps",
        r#"
            Generate explicit checks around native division instructions to avoid
            their trapping.

            This is primarily used by SpiderMonkey which doesn't install a signal
            handler for SIGFPE, but expects a SIGILL trap for division by zero.

            On ISAs like ARM where the native division instructions don't trap,
            this setting has no effect - explicit checks are always inserted.
            "#,
        false,
    );

    settings.add_bool(
        "enable_float",
        r#"
            Enable the use of floating-point instructions

            Disabling use of floating-point instructions is not yet implemented.
            "#,
        true,
    );

    settings.add_bool(
        "enable_nan_canonicalization",
        r#"
            Enable NaN canonicalization

            This replaces NaNs with a single canonical value, for users requiring
            entirely deterministic WebAssembly computation. This is not required
            by the WebAssembly spec, so it is not enabled by default.
            "#,
        false,
    );

    settings.add_bool("enable_simd", "Enable the use of SIMD instructions.", true);

    settings.add_bool(
        "enable_atomics",
        "Enable the use of atomic instructions",
        true,
    );

    // Settings specific to the `baldrdash` calling convention.

    settings.add_num(
        "baldrdash_prologue_words",
        r#"
            Number of pointer-sized words pushed by the baldrdash prologue.

            Functions with the `baldrdash` calling convention don't generate their
            own prologue and epilogue. They depend on externally generated code
            that pushes a fixed number of words in the prologue and restores them
            in the epilogue.

            This setting configures the number of pointer-sized words pushed on the
            stack when the Cranelift-generated code is entered. This includes the
            pushed return address on x86.
            "#,
        0,
    );

    // BaldrMonkey requires that not-yet-relocated function addresses be encoded
    // as all-ones bitpatterns.
    settings.add_bool(
        "allones_funcaddrs",
        "Emit not-yet-relocated function addresses as all-ones bit patterns.",
        false,
    );

    // Stack probing options.

    settings.add_bool(
        "probestack_enabled",
        r#"
            Enable the use of stack probes, for calling conventions which support this
            functionality.
            "#,
        true,
    );

    settings.add_bool(
        "probestack_func_adjusts_sp",
        r#"
            Set this to true of the stack probe function modifies the stack pointer
            itself.
            "#,
        false,
    );

    settings.add_num(
        "probestack_size_log2",
        r#"
            The log2 of the size of the stack guard region.

            Stack frames larger than this size will have stack overflow checked
            by calling the probestack function.

            The default is 12, which translates to a size of 4096.
            "#,
        12,
    );

    // Jump table options.

    settings.add_bool(
        "jump_tables_enabled",
        "Enable the use of jump tables in generated machine code.",
        true,
    );

    settings.finish()
}
