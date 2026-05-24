use test_programs::p3::wasi;
use test_programs::p3::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags,
};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let preopens = wasi::filesystem::preopens::get_directories();
        let (dir, _) = &preopens[0];

        test_path_link(dir).await;
        Ok(())
    }
}

fn main() {
    unreachable!()
}

async fn test_path_link(dir: &Descriptor) {
    // Create a file
    dir.open_at(
        PathFlags::empty(),
        "file".to_string(),
        OpenFlags::CREATE,
        DescriptorFlags::empty(),
    )
    .await
    .expect("create file");

    // Open a fresh descriptor to the file
    let file = dir
        .open_at(
            PathFlags::empty(),
            "file".to_string(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .await
        .expect("open file");

    // Create a link in the same directory and verify they refer to the same object
    dir.link_at(
        PathFlags::empty(),
        "file".to_string(),
        dir,
        "link".to_string(),
    )
    .await
    .expect("creating a link in the same directory");

    let link = dir
        .open_at(
            PathFlags::empty(),
            "link".to_string(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .await
        .expect("open link");

    assert!(
        file.is_same_object(&link).await,
        "file and link should be the same object"
    );
    let file_hash = file.metadata_hash().await.expect("file metadata hash");
    let link_hash = link.metadata_hash().await.expect("link metadata hash");
    assert_eq!(file_hash.lower, link_hash.lower, "metadata hash lower should be equal");
    assert_eq!(file_hash.upper, link_hash.upper, "metadata hash upper should be equal");

    drop(link);
    dir.unlink_file_at("link".to_string())
        .await
        .expect("removing a link");

    // Create a link in a different directory and verify they refer to the same object
    dir.create_directory_at("subdir".to_string())
        .await
        .expect("creating a subdirectory");
    let subdir = dir
        .open_at(
            PathFlags::empty(),
            "subdir".to_string(),
            OpenFlags::DIRECTORY,
            DescriptorFlags::MUTATE_DIRECTORY | DescriptorFlags::READ,
        )
        .await
        .expect("open subdir directory");

    dir.link_at(
        PathFlags::empty(),
        "file".to_string(),
        &subdir,
        "link".to_string(),
    )
    .await
    .expect("creating a link in subdirectory");

    let link = subdir
        .open_at(
            PathFlags::empty(),
            "link".to_string(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .await
        .expect("open link in subdir");

    assert!(
        file.is_same_object(&link).await,
        "file and link in subdir should be the same object"
    );

    drop(link);
    subdir
        .unlink_file_at("link".to_string())
        .await
        .expect("removing a link");
    drop(subdir);
    dir.remove_directory_at("subdir".to_string())
        .await
        .expect("removing a subdirectory");

    // Create a link to a path that already exists
    dir.open_at(
        PathFlags::empty(),
        "link".to_string(),
        OpenFlags::CREATE,
        DescriptorFlags::empty(),
    )
    .await
    .expect("create link file");

    let err = dir
        .link_at(
            PathFlags::empty(),
            "file".to_string(),
            dir,
            "link".to_string(),
        )
        .await
        .expect_err("creating a link to existing path should fail");
    assert!(
        matches!(err, ErrorCode::Exist),
        "expected Exist error, got {err:?}"
    );
    dir.unlink_file_at("link".to_string())
        .await
        .expect("removing a file");

    // Create a link to itself
    let err = dir
        .link_at(
            PathFlags::empty(),
            "file".to_string(),
            dir,
            "file".to_string(),
        )
        .await
        .expect_err("creating a link to itself should fail");
    assert!(
        matches!(err, ErrorCode::Exist),
        "expected Exist error, got {err:?}"
    );

    // Create a link where target is a directory
    dir.create_directory_at("link".to_string())
        .await
        .expect("creating a dir");

    let err = dir
        .link_at(
            PathFlags::empty(),
            "file".to_string(),
            dir,
            "link".to_string(),
        )
        .await
        .expect_err("creating a link where target is a directory should fail");
    assert!(
        matches!(err, ErrorCode::Exist),
        "expected Exist error, got {err:?}"
    );
    dir.remove_directory_at("link".to_string())
        .await
        .expect("removing a dir");

    // Create a link to a directory
    dir.create_directory_at("subdir".to_string())
        .await
        .expect("creating a subdirectory");

    let err = dir
        .link_at(
            PathFlags::empty(),
            "subdir".to_string(),
            dir,
            "link".to_string(),
        )
        .await
        .expect_err("creating a link to a directory should fail");
    assert!(
        matches!(err, ErrorCode::NotPermitted | ErrorCode::Access),
        "expected NotPermitted or Access error, got {err:?}"
    );
    dir.remove_directory_at("subdir".to_string())
        .await
        .expect("removing a subdirectory");

    // Create a link to a file with trailing slash
    let err = dir
        .link_at(
            PathFlags::empty(),
            "file".to_string(),
            dir,
            "link/".to_string(),
        )
        .await
        .expect_err("creating a link to a file with trailing slash should fail");
    assert!(
        matches!(err, ErrorCode::NoEntry),
        "expected NoEntry error, got {err:?}"
    );

    // Clean up
    drop(file);
    dir.unlink_file_at("file".to_string())
        .await
        .expect("removing a file");
}
