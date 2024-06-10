#[test]
fn filetests() -> anyhow::Result<()> {
    // Run all the filetests in the following directories.
    cranelift_filetests::run(false, false, &["filetests".into(), "docs".into()])?;
    Ok(())
}
