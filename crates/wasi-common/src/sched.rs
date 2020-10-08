use crate::clock::Timestamp;
use crate::ctx::WasiCtx;
use crate::entry::EntryHandle;
use crate::fs::Fd;
use crate::handle::{Filesize, HandleRights, Rights};
pub use crate::wasi::types::{Eventrwflags, Userdata};
use crate::Error;
use std::cell::Cell;
use std::convert::TryInto;
use std::time::{Duration, SystemTime};

pub struct RwSubscription {
    pub handle: EntryHandle,
    // Interior mutation makes RwSubscription a reasonable abstraction - otherwise the borrow
    // checker often gets in the way
    status: Cell<Option<Result<(Filesize, Eventrwflags), Error>>>,
}

impl RwSubscription {
    pub fn new(handle: EntryHandle) -> Self {
        Self {
            handle,
            status: Cell::new(None),
        }
    }
    pub fn complete(&self, size: Filesize, flags: Eventrwflags) {
        self.status.set(Some(Ok((size, flags))))
    }
    pub fn error(&self, error: Error) {
        self.status.set(Some(Err(error)))
    }
    pub fn result(self) -> Option<Result<(Filesize, Eventrwflags), Error>> {
        self.status.into_inner()
    }
}

pub struct TimerSubscription {
    pub deadline: SystemTime,
}

impl TimerSubscription {
    pub fn from_absolute(ts: Timestamp) -> Result<Self, Error> {
        let epoch = SystemTime::UNIX_EPOCH;
        // this line better not be in production by the year 2200:
        let ts: u64 = ts.try_into().map_err(|_| Error::TooBig)?;
        let deadline = epoch
            .checked_add(Duration::from_nanos(ts))
            .ok_or(Error::TooBig)?;
        Ok(Self { deadline })
    }
    pub fn from_relative(ts: Timestamp) -> Result<Self, Error> {
        let now = SystemTime::now();
        // this line better not be in production by the year 2200:
        let ts: u64 = ts.try_into().map_err(|_| Error::TooBig)?;
        let deadline = now
            .checked_add(Duration::from_nanos(ts))
            .ok_or(Error::TooBig)?;
        Ok(Self { deadline })
    }
    /// Ok indicates deadline in the past, None indicates deadline yet to be reached
    pub fn result(&self) -> Option<Result<(), Error>> {
        // deadline is only an Ok(Duration) from now if now is in past:
        if self.deadline.duration_since(SystemTime::now()).is_ok() {
            Some(Ok(()))
        } else {
            None
        }
    }
}

pub enum Subscription {
    Read(RwSubscription),
    Write(RwSubscription),
    Timer(TimerSubscription),
}

pub struct SubscriptionSet<'a> {
    pub subs: Vec<&'a Subscription>,
}

impl<'a> SubscriptionSet<'a> {
    pub fn earliest_deadline(&self) -> Option<SystemTime> {
        self.subs
            .iter()
            .filter_map(|s| match s {
                Subscription::Timer(ts) => Some(ts.deadline),
                _ => None,
            })
            .fold(None, |early, ts| {
                if let Some(early) = early {
                    Some(early.min(ts))
                } else {
                    Some(ts)
                }
            })
    }
}

pub enum SubscriptionResult {
    Read(Result<(Filesize, Eventrwflags), Error>),
    Write(Result<(Filesize, Eventrwflags), Error>),
    Timer(Result<(), Error>),
}

impl SubscriptionResult {
    fn from_subscription(s: Subscription) -> Option<SubscriptionResult> {
        match s {
            Subscription::Read(s) => s.result().map(SubscriptionResult::Read),
            Subscription::Write(s) => s.result().map(SubscriptionResult::Write),
            Subscription::Timer(s) => s.result().map(SubscriptionResult::Timer),
        }
    }
}

pub struct Poll<'a> {
    ctx: &'a WasiCtx,
    subs: Vec<(Subscription, Userdata)>,
}

impl<'a> Poll<'a> {
    pub fn new(ctx: &'a WasiCtx) -> Self {
        Poll {
            ctx,
            subs: Vec::new(),
        }
    }

    pub fn subscribe_timer(
        &mut self,
        at: Timestamp,
        ud: Userdata,
        is_absolute: bool,
    ) -> Result<(), Error> {
        self.subs.push((
            Subscription::Timer(if is_absolute {
                TimerSubscription::from_absolute(at)?
            } else {
                TimerSubscription::from_relative(at)?
            }),
            ud,
        ));
        Ok(())
    }

    pub fn subscribe_relative_time(&mut self, at: Timestamp, ud: Userdata) -> Result<(), Error> {
        self.subs.push((
            Subscription::Timer(TimerSubscription::from_relative(at)?),
            ud,
        ));
        Ok(())
    }

    pub fn subscribe_absolute_time(&mut self, at: Timestamp, ud: Userdata) -> Result<(), Error> {
        self.subs.push((
            Subscription::Timer(TimerSubscription::from_absolute(at)?),
            ud,
        ));
        Ok(())
    }

    pub fn subscribe_fd_read(&mut self, fd: Fd, ud: Userdata) -> Result<(), Error> {
        let entry = self.ctx.get_entry(fd)?;
        let required_rights = HandleRights::from_base(Rights::FD_READ | Rights::POLL_FD_READWRITE);
        let handle = entry.as_handle(&required_rights)?;
        self.subs
            .push((Subscription::Read(RwSubscription::new(handle)), ud));
        Ok(())
    }

    pub fn subscribe_fd_write(&mut self, fd: Fd, ud: Userdata) -> Result<(), Error> {
        let entry = self.ctx.get_entry(fd)?;
        let required_rights = HandleRights::from_base(Rights::FD_WRITE | Rights::POLL_FD_READWRITE);
        let handle = entry.as_handle(&required_rights)?;
        self.subs
            .push((Subscription::Write(RwSubscription::new(handle)), ud));
        Ok(())
    }

    pub fn subscriptions(&mut self) -> SubscriptionSet {
        SubscriptionSet {
            subs: self.subs.iter().map(|(s, _ud)| s).collect(),
        }
    }

    pub fn results(self) -> Vec<(SubscriptionResult, Userdata)> {
        self.subs
            .into_iter()
            .filter_map(|(s, ud)| SubscriptionResult::from_subscription(s).map(|r| (r, ud)))
            .collect()
    }
}
