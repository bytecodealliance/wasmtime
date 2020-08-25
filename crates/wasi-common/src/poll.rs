use crate::entry::EntryHandle;
use crate::wasi::types;

pub(crate) use crate::sys::poll::*;

#[derive(Debug, Copy, Clone)]
pub(crate) struct ClockEventData {
    pub(crate) delay: u128, // delay is expressed in nanoseconds
    pub(crate) userdata: types::Userdata,
}

#[derive(Debug)]
pub(crate) struct FdEventData {
    pub(crate) handle: EntryHandle,
    pub(crate) r#type: types::Eventtype,
    pub(crate) userdata: types::Userdata,
}
