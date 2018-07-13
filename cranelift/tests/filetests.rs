extern crate cranelift_filetests;

#[test]
fn filetests() {
    // Run all the filetests in the following directories.
    cranelift_filetests::run(false, &["filetests".into(), "docs".into()]).expect("test harness");
}
