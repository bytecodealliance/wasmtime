use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::Descriptor;

fn test_path_rename_trailing_slashes(dir: &Descriptor) {
    // Test renaming a directory with a trailing slash in the name.
    dir.create_directory_at("source")
        .expect("creating a directory");
    dir.rename_at("source/", dir, "target")
        .expect("renaming a directory with a trailing slash in the source name");
    dir.rename_at("target", dir, "source/")
        .expect("renaming a directory with a trailing slash in the destination name");
    dir.rename_at("source/", dir, "target/")
        .expect("renaming a directory with a trailing slash in the source and destination names");
    dir.rename_at("target", dir, "source")
        .expect("renaming a directory with no trailing slashes at all should work");
    dir.remove_directory_at("source")
        .expect("removing the directory");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_rename_trailing_slashes(dir)
}
