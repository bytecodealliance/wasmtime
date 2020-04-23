#[path = "../linux/osfile.rs"]
pub(crate) mod osfile;
#[path = "../linux/path.rs"]
pub(crate) mod path;

pub(crate) const O_RSYNC: yanix::file::OFlag = yanix::file::OFlag::RSYNC;
