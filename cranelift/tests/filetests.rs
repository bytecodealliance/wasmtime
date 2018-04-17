extern crate cretonne_filetests;

#[test]
fn filetests() {
    // Run all the filetests in the following directories.
    cretonne_filetests::run(false, &["filetests".into(), "docs".into()]).expect("test harness");
}
