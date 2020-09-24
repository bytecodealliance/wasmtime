#[test]
fn filetests() {
    // Run all the filetests in the following directories.
    cranelift_filetests::run(false, false, &["filetests".into(), "docs".into()])
        .expect("test harness");
}
