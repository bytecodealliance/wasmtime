#[path = "../linux/oshandle.rs"]
pub(crate) mod oshandle;
#[path = "../linux/path.rs"]
pub(crate) mod path;

pub(crate) const O_RSYNC: yanix::file::OFlag = yanix::file::OFlag::RSYNC;
