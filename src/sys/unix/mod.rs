pub mod fdentry;
mod host_impl;
pub mod hostcalls;

pub fn dev_null() -> std::fs::File {
    std::fs::File::open("/dev/null").expect("failed to open /dev/null")
}
