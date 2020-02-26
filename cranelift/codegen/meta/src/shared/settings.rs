use crate::cdsl::settings::{SettingGroup, SettingGroupBuilder};

pub(crate) fn define() -> SettingGroup {
    let mut settings = SettingGroupBuilder::new("shared");

    settings.add_enum(
        "opt_level",
        r#"
        Optimization level:

        - none: Minimise compile time by disabling most optimizations.
        - speed: Generate the fastest possible code
        - speed_and_size: like "speed", but also perform transformations
          aimed at reducing code size.
        "#,
        vec!["none", "speed", "speed_and_size"],
    );

    settings.add_bool(
        "enable_verifier",
        r#"
        Run the Cranelift IR verifier at strategic times during compilation.

        This makes compilation slower but catches many bugs. The verifier is always enabled by
        default, which is useful during development.
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
        "use_colocated_libcalls",
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

    settings.add_bool(
        "enable_pinned_reg",
        r#"Enable the use of the pinned register.

        This register is excluded from register allocation, and is completely under the control of
        the end-user. It is possible to read it via the get_pinned_reg instruction, and to set it
        with the set_pinned_reg instruction.
        "#,
        false,
    );

    settings.add_bool(
        "use_pinned_reg_as_heap_base",
        r#"Use the pinned register as the heap base.

        Enabling this requires the enable_pinned_reg setting to be set to true. It enables a custom
        legalization of the `heap_addr` instruction so it will use the pinned register as the heap
        base, instead of fetching it from a global value.

        Warning! Enabling this means that the pinned register *must* be maintained to contain the
        heap base address at all times, during the lifetime of a function. Using the pinned
        register for other purposes when this is set is very likely to cause crashes.
        "#,
        false,
    );

    settings.add_bool("enable_simd", "Enable the use of SIMD instructions.", false);

    settings.add_bool(
        "enable_atomics",
        "Enable the use of atomic instructions",
        true,
    );

    settings.add_bool(
        "enable_safepoints",
        r#"
            Enable safepoint instruction insertions.

            This will allow the emit_stackmaps() function to insert the safepoint
            instruction on top of calls and interrupt traps in order to display the
            live reference values at that point in the program.
            "#,
        false,
    );

    settings.add_enum(
        "tls_model",
        r#"
            Defines the model used to perform TLS accesses.
        "#,
        vec!["none", "elf_gd", "macho", "coff"],
    );

    // Settings specific to the `baldrdash` calling convention.

    settings.add_enum(
        "libcall_call_conv",
        r#"
            Defines the calling convention to use for LibCalls call expansion,
            since it may be different from the ISA default calling convention.

            The default value is to use the same calling convention as the ISA
            default calling convention.

            This list should be kept in sync with the list of calling
            conventions available in isa/call_conv.rs.
        "#,
        vec![
            "isa_default",
            "fast",
            "cold",
            "system_v",
            "windows_fastcall",
            "baldrdash_system_v",
            "baldrdash_windows",
            "probestack",
        ],
    );

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
        "emit_all_ones_funcaddrs",
        "Emit not-yet-relocated function addresses as all-ones bit patterns.",
        false,
    );

    // Stack probing options.

    settings.add_bool(
        "enable_probestack",
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
        "enable_jump_tables",
        "Enable the use of jump tables in generated machine code.",
        true,
    );

    settings.build()
}
