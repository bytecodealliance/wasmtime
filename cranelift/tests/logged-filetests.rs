/// This test runs the filetests with logging enabled.
///
/// In order to avoid issues like [#10529] and similar, we re-run the filetests
/// with logging enabled. This pretty-prints instructions prior to register
/// allocation, when they may not have real HW registers assigned. Ideally this
/// test is temporary while we work out details in the `cranelift-assembler-x64`
/// crate (TODO).
///
/// [#10529]: https://github.com/bytecodealliance/wasmtime/issues/10529
#[test]
fn logged_filetests() -> anyhow::Result<()> {
    let _ = pretty_env_logger::formatted_builder()
        .filter_module(
            "cranelift_codegen::machinst::lower",
            log::LevelFilter::Trace,
        )
        .is_test(true)
        .init();
    cranelift_filetests::run(false, false, &["filetests".into(), "docs".into()])?;
    Ok(())
}
