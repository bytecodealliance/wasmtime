pub use tempfile::TempDir;

pub fn tmpdir() -> TempDir {
    tempfile::tempdir().expect("expected to be able to create a temporary directory")
}
