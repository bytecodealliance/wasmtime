use crate::entry::EntryHandle;
pub use crate::wasi::types::{
    Clockid, Errno, Event, EventFdReadwrite, Eventrwflags, Eventtype, Subclockflags,
    SubscriptionClock, Timestamp, Userdata,
};
#[derive(Debug, Copy, Clone)]
pub struct ClockEventData {
    pub delay: u128, // delay is expressed in nanoseconds
    pub userdata: Userdata,
}

#[derive(Debug)]
pub struct FdEventData {
    pub handle: EntryHandle,
    pub r#type: Eventtype,
    pub userdata: Userdata,
}
