#[test]
#[cfg_attr(feature = "experimental_x64", should_panic)] // TODO #2079
fn filetests() {
    // Run all the filetests in the following directories.
    cranelift_filetests::run(false, false, &["filetests".into(), "docs".into()])
        .expect("test harness");
}
