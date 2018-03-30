extern crate cton_filetests;

#[test]
fn filetests() {
    // Run all the filetests in the following directories.
    cton_filetests::run(false, &["filetests".into(), "docs".into()]).expect("test harness");
}
