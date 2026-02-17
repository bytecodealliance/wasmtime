use test_programs::p3::wasi;
use test_programs::p3::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, DescriptorType, DirectoryEntry, OpenFlags, PathFlags,
};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let preopens = wasi::filesystem::preopens::get_directories();
        let (dir, _) = &preopens[0];

        test_readdir(dir).await;
        test_readdir_lots(dir).await;
        Ok(())
    }
}

fn main() {
    unreachable!()
}

async fn read_dir(dir: &Descriptor) -> Vec<DirectoryEntry> {
    let (dirs, result) = dir.read_directory();
    let mut dirs = dirs.collect().await;
    result.await.unwrap();
    dirs.sort_by_key(|d| d.name.clone());
    dirs
}

async fn assert_empty_dir(dir: &Descriptor) {
    let dirs = read_dir(dir).await;
    assert_eq!(dirs.len(), 0);
}

async fn test_readdir(dir: &Descriptor) {
    // Check the behavior in an empty directory
    assert_empty_dir(dir).await;

    dir.open_at(
        PathFlags::empty(),
        "file".to_string(),
        OpenFlags::CREATE,
        DescriptorFlags::READ | DescriptorFlags::WRITE,
    )
    .await
    .unwrap();

    dir.create_directory_at("nested".to_string()).await.unwrap();
    let nested = dir
        .open_at(
            PathFlags::empty(),
            "nested".to_string(),
            OpenFlags::DIRECTORY,
            DescriptorFlags::empty(),
        )
        .await
        .unwrap();

    let entries = read_dir(dir).await;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "file");
    assert_eq!(entries[0].type_, DescriptorType::RegularFile);
    assert_eq!(entries[1].name, "nested");
    assert_eq!(entries[1].type_, DescriptorType::Directory);

    assert_empty_dir(&nested).await;
    drop(nested);

    dir.unlink_file_at("file".to_string()).await.unwrap();
    dir.remove_directory_at("nested".to_string()).await.unwrap();
}

async fn test_readdir_lots(dir: &Descriptor) {
    for count in 0..1000 {
        dir.open_at(
            PathFlags::empty(),
            format!("file.{count}"),
            OpenFlags::CREATE,
            DescriptorFlags::READ | DescriptorFlags::WRITE,
        )
        .await
        .expect("failed to create file");
    }

    assert_eq!(read_dir(dir).await.len(), 1000);

    for count in 0..1000 {
        dir.unlink_file_at(format!("file.{count}"))
            .await
            .expect("removing a file");
    }
}
