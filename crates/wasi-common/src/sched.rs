use crate::ctx::WasiCtx;
use crate::entry::EntryHandle;
use crate::fs::Fd;
use crate::handle::{HandleRights, Rights};
use crate::Result;

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

pub struct PollBuilder<'a> {
    ctx: &'a WasiCtx,
    timeout: Option<ClockEventData>,
    fd_events: Vec<FdEventData>,
}

impl<'a> PollBuilder<'a> {
    pub fn new(ctx: &'a WasiCtx) -> Self {
        PollBuilder {
            ctx,
            timeout: None,
            fd_events: Vec::new(),
        }
    }

    pub fn subscribe_clock(&mut self, clock: SubscriptionClock, userdata: Userdata) -> Result<()> {
        let delay = crate::sys::clock::to_relative_ns_delay(&clock)?;
        let current = ClockEventData { delay, userdata };
        let timeout = self.timeout.get_or_insert(current);
        if current.delay < timeout.delay {
            *timeout = current;
        }
        Ok(())
    }

    pub fn subscribe_fd_read(&mut self, fd: Fd, userdata: Userdata) -> Result<()> {
        let entry = self.ctx.get_entry(fd)?;
        let required_rights = HandleRights::from_base(Rights::FD_READ | Rights::POLL_FD_READWRITE);
        let handle = entry.as_handle(&required_rights)?;
        self.fd_events.push(FdEventData {
            handle,
            r#type: Eventtype::FdRead,
            userdata,
        });
        Ok(())
    }

    pub fn subscribe_fd_write(&mut self, fd: Fd, userdata: Userdata) -> Result<()> {
        let entry = self.ctx.get_entry(fd)?;
        let required_rights = HandleRights::from_base(Rights::FD_WRITE | Rights::POLL_FD_READWRITE);
        let handle = entry.as_handle(&required_rights)?;
        self.fd_events.push(FdEventData {
            handle,
            r#type: Eventtype::FdWrite,
            userdata,
        });
        Ok(())
    }

    pub fn poll(self) -> Result<Vec<Event>> {
        crate::sys::poll::oneoff(self.timeout, self.fd_events)
    }
}
