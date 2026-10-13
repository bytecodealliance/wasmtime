use test_programs::p3::wasi;
use test_programs::p3::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags,
};
use test_programs::preview1::config;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        // This test program requires a preopen at the path "sandbox", next to
        // an "outside" directory which isn't preopened.
        let preopens = wasi::filesystem::preopens::get_directories();
        let (dir, _) = preopens
            .iter()
            .find(|(_, path)| path == "sandbox")
            .expect("find preopen named sandbox");

        test_path_rename_trailing_slashes(dir).await;
        Ok(())
    }
}

fn main() {
    unreachable!()
}

async fn assert_not_directory(dir: &Descriptor, old_path: &str, new_path: &str) {
    let err = match dir
        .rename_at(old_path.to_string(), dir, new_path.to_string())
        .await
    {
        Ok(()) => panic!("renaming {old_path:?} to {new_path:?} should fail"),
        Err(err) => err,
    };
    assert!(
        matches!(err, ErrorCode::NotDirectory),
        "unexpected error {err:?} renaming {old_path:?} to {new_path:?}"
    );
}

async fn test_path_rename_trailing_slashes(dir: &Descriptor) {
    // A symlink to a directory outside of the preopen with a trailing slash on
    // either path isn't followed, so the directory isn't moved into the
    // preopen. The host checks that it's still in place. Windows can't create
    // such a symlink in the first place.
    match dir
        .symlink_at("../outside".to_string(), "up".to_string())
        .await
    {
        Ok(()) => {
            assert_not_directory(dir, "up/", "stolen").await;
            assert_not_directory(dir, "up", "stolen/").await;
            dir.unlink_file_at("up".to_string())
                .await
                .expect("removing a symlink");
        }
        Err(ErrorCode::NotPermitted) if config().errno_expect_windows() => {}
        Err(err) => panic!("creating a symlink: {err:?}"),
    }

    // A symlink to a directory inside of the preopen with a trailing slash on
    // either path isn't followed either.
    dir.create_directory_at("dir".to_string())
        .await
        .expect("creating a directory");
    dir.symlink_at("dir".to_string(), "link".to_string())
        .await
        .expect("creating a symlink");
    assert_not_directory(dir, "link/", "target").await;
    assert_not_directory(dir, "link", "target/").await;
    dir.stat_at(PathFlags::empty(), "dir".to_string())
        .await
        .expect("the directory wasn't renamed");
    dir.stat_at(PathFlags::empty(), "link".to_string())
        .await
        .expect("the symlink wasn't renamed");
    dir.unlink_file_at("link".to_string())
        .await
        .expect("removing a symlink");
    dir.rename_at("dir/".to_string(), dir, "renamed/".to_string())
        .await
        .expect("renaming a directory");
    dir.remove_directory_at("renamed".to_string())
        .await
        .expect("removing a directory");

    // A file with a trailing slash on either path isn't a directory.
    dir.open_at(
        PathFlags::empty(),
        "file".to_string(),
        OpenFlags::CREATE,
        DescriptorFlags::empty(),
    )
    .await
    .expect("creating a file");
    assert_not_directory(dir, "file/", "target").await;
    assert_not_directory(dir, "file", "target/").await;
    dir.stat_at(PathFlags::empty(), "file".to_string())
        .await
        .expect("the file wasn't renamed");
    dir.unlink_file_at("file".to_string())
        .await
        .expect("removing a file");
}
