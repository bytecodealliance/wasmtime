use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Advice, Descriptor, DescriptorType, ErrorCode};

fn test_fd_dir_ops(dir: &Descriptor) {
    assert_eq!(
        dir.get_type().expect("failed get_type"),
        DescriptorType::Directory
    );

    // On posix, this fails with ERRNO_ISDIR:
    let r = dir.read_via_stream(0).err();
    assert_eq!(r, Some(ErrorCode::IsDirectory), "read_via_stream error");

    // Same behavior as specified by POSIX:
    let r = dir.write_via_stream(0).err();
    assert_eq!(r, Some(ErrorCode::BadDescriptor), "write_via_stream error");

    let r = dir.append_via_stream().err();
    assert_eq!(r, Some(ErrorCode::BadDescriptor), "append_via_stream error");

    // posix_fadvise(dirfd, 0, 0, POSIX_FADV_DONTNEED) will return 0 on linux.
    // not available on mac os.
    let r = dir.advise(0, 0, Advice::DontNeed).err();
    assert_eq!(r, Some(ErrorCode::BadDescriptor), "advise error");

    // ftruncate(dirfd, 1) will fail with errno EINVAL on posix.
    // here, we fail with bad-descriptor instead:
    let r = dir.set_size(0).err();
    assert_eq!(r, Some(ErrorCode::BadDescriptor), "set_size error");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    test_fd_dir_ops(dir);
}
