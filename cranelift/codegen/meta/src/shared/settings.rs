use crate::cdsl::settings::{SettingGroup, SettingGroupBuilder};

pub(crate) fn define() -> SettingGroup {
    let mut settings = SettingGroupBuilder::new("shared");

    settings.add_bool(
        "regalloc_checker",
        "Enable the symbolic checker for register allocation.",
        r#"
            This performs a verification that the register allocator preserves
            equivalent dataflow with respect to the original (pre-regalloc)
            program. This analysis is somewhat expensive. However, if it succeeds,
            it provides independent evidence (by a carefully-reviewed, from-first-principles
            analysis) that no regalloc bugs were triggered for the particular compilations
            performed. This is a valuable assurance to have as regalloc bugs can be
            very dangerous and difficult to debug.
        "#,
        false,
    );

    settings.add_enum(
        "opt_level",
        "Optimization level for generated code.",
        r#"
            Supported levels:

            - `none`: Minimise compile time by disabling most optimizations.
            - `speed`: Generate the fastest possible code
            - `speed_and_size`: like "speed", but also perform transformations aimed at reducing code size.
        "#,
        vec!["none", "speed", "speed_and_size"],
    );

    settings.add_bool(
        "enable_verifier",
        "Run the Cranelift IR verifier at strategic times during compilation.",
        r#"
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
        "Enable Position-Independent Code generation.",
        "",
        false,
    );

    settings.add_bool(
        "use_colocated_libcalls",
        "Use colocated libcalls.",
        r#"
            Generate code that assumes that libcalls can be declared "colocated",
            meaning they will be defined along with the current function, such that
            they can use more efficient addressing.
        "#,
        false,
    );

    settings.add_bool(
        "avoid_div_traps",
        "Generate explicit checks around native division instructions to avoid their trapping.",
        r#"
            This is primarily used by SpiderMonkey which doesn't install a signal
            handler for SIGFPE, but expects a SIGILL trap for division by zero.

            On ISAs like ARM where the native division instructions don't trap,
            this setting has no effect - explicit checks are always inserted.
        "#,
        false,
    );

    settings.add_bool(
        "enable_float",
        "Enable the use of floating-point instructions.",
        r#"
            Disabling use of floating-point instructions is not yet implemented.
        "#,
        true,
    );

    settings.add_bool(
        "enable_nan_canonicalization",
        "Enable NaN canonicalization.",
        r#"
            This replaces NaNs with a single canonical value, for users requiring
            entirely deterministic WebAssembly computation. This is not required
            by the WebAssembly spec, so it is not enabled by default.
        "#,
        false,
    );

    settings.add_bool(
        "enable_pinned_reg",
        "Enable the use of the pinned register.",
        r#"
            This register is excluded from register allocation, and is completely under the control of
            the end-user. It is possible to read it via the get_pinned_reg instruction, and to set it
            with the set_pinned_reg instruction.
        "#,
        false,
    );

    settings.add_bool(
        "use_pinned_reg_as_heap_base",
        "Use the pinned register as the heap base.",
        r#"
            Enabling this requires the enable_pinned_reg setting to be set to true. It enables a custom
            legalization of the `heap_addr` instruction so it will use the pinned register as the heap
            base, instead of fetching it from a global value.

            Warning! Enabling this means that the pinned register *must* be maintained to contain the
            heap base address at all times, during the lifetime of a function. Using the pinned
            register for other purposes when this is set is very likely to cause crashes.
        "#,
        false,
    );

    settings.add_bool(
        "enable_simd",
        "Enable the use of SIMD instructions.",
        "",
        false,
    );

    settings.add_bool(
        "enable_atomics",
        "Enable the use of atomic instructions",
        "",
        true,
    );

    settings.add_bool(
        "enable_safepoints",
        "Enable safepoint instruction insertions.",
        r#"
            This will allow the emit_stack_maps() function to insert the safepoint
            instruction on top of calls and interrupt traps in order to display the
            live reference values at that point in the program.
        "#,
        false,
    );

    settings.add_enum(
        "tls_model",
        "Defines the model used to perform TLS accesses.",
        "",
        vec!["none", "elf_gd", "macho", "coff"],
    );

    // Settings specific to the `baldrdash` calling convention.

    settings.add_enum(
        "libcall_call_conv",
        "Defines the calling convention to use for LibCalls call expansion.",
        r#"
            This may be different from the ISA default calling convention.

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
            "apple_aarch64",
            "baldrdash_system_v",
            "baldrdash_windows",
            "baldrdash_2020",
            "probestack",
        ],
    );

    settings.add_num(
        "baldrdash_prologue_words",
        "Number of pointer-sized words pushed by the baldrdash prologue.",
        r#"
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

    settings.add_bool(
        "enable_llvm_abi_extensions",
        "Enable various ABI extensions defined by LLVM's behavior.",
        r#"
            In some cases, LLVM's implementation of an ABI (calling convention)
            goes beyond a standard and supports additional argument types or
            behavior. This option instructs Cranelift codegen to follow LLVM's
            behavior where applicable.

            Currently, this applies only to Windows Fastcall on x86-64, and
            allows an `i128` argument to be spread across two 64-bit integer
            registers. The Fastcall implementation otherwise does not support
            `i128` arguments, and will panic if they are present and this
            option is not set.
        "#,
        false,
    );

    settings.add_bool(
        "unwind_info",
        "Generate unwind information.",
        r#"
            This increases metadata size and compile time, but allows for the
            debugger to trace frames, is needed for GC tracing that relies on
            libunwind (such as in Wasmtime), and is unconditionally needed on
            certain platforms (such as Windows) that must always be able to unwind.
          "#,
        true,
    );

    settings.add_bool(
        "machine_code_cfg_info",
        "Generate CFG metadata for machine code.",
        r#"
            This increases metadata size and compile time, but allows for the
            embedder to more easily post-process or analyze the generated
            machine code. It provides code offsets for the start of each
            basic block in the generated machine code, and a list of CFG
            edges (with blocks identified by start offsets) between them.
            This is useful for, e.g., machine-code analyses that verify certain
            properties of the generated code.
        "#,
        false,
    );

    // BaldrMonkey requires that not-yet-relocated function addresses be encoded
    // as all-ones bitpatterns.
    settings.add_bool(
        "emit_all_ones_funcaddrs",
        "Emit not-yet-relocated function addresses as all-ones bit patterns.",
        "",
        false,
    );

    // Stack probing options.

    settings.add_bool(
        "enable_probestack",
        "Enable the use of stack probes for supported calling conventions.",
        "",
        true,
    );

    settings.add_bool(
        "probestack_func_adjusts_sp",
        "Enable if the stack probe adjusts the stack pointer.",
        "",
        false,
    );

    settings.add_num(
        "probestack_size_log2",
        "The log2 of the size of the stack guard region.",
        r#"
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
        "",
        true,
    );

    // Spectre options.

    settings.add_bool(
        "enable_heap_access_spectre_mitigation",
        "Enable Spectre mitigation on heap bounds checks.",
        r#"
            This is a no-op for any heap that needs no bounds checks; e.g.,
            if the limit is static and the guard region is large enough that
            the index cannot reach past it.

            This option is enabled by default because it is highly
            recommended for secure sandboxing. The embedder should consider
            the security implications carefully before disabling this option.
        "#,
        true,
    );

    settings.add_bool(
        "enable_table_access_spectre_mitigation",
        "Enable Spectre mitigation on table bounds checks.",
        r#"
            This option uses a conditional move to ensure that when a table
            access index is bounds-checked and a conditional branch is used
            for the out-of-bounds case, a misspeculation of that conditional
            branch (falsely predicted in-bounds) will select an in-bounds
            index to load on the speculative path.
            
            This option is enabled by default because it is highly
            recommended for secure sandboxing. The embedder should consider
            the security implications carefully before disabling this option.
        "#,
        true,
    );

    settings.build()
}
