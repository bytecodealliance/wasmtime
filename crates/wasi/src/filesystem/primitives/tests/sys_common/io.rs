pub use tempfile::TempDir;

#[allow(unused)]
pub fn tmpdir() -> TempDir {
    tempfile::tempdir().expect("expected to be able to create a temporary directory")
}
