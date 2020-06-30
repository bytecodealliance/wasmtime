#[path = "../linux/osdir.rs"]
pub(crate) mod osdir;
#[path = "../linux/path.rs"]
pub(crate) mod path;

pub(crate) const O_RSYNC: yanix::file::OFlags = yanix::file::OFlags::RSYNC;
