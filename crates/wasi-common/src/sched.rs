use crate::ctx::WasiCtx;
use crate::entry::EntryHandle;
use crate::fs::Fd;
use crate::handle::{Filesize, HandleRights, Rights};
use crate::Error;
use std::convert::TryInto;
use std::time::{Duration, SystemTime};

pub use crate::wasi::types::{Eventrwflags, Timestamp, Userdata};

pub enum SchedResult<A, E> {
    /// The subscribed event did not happen
    None,
    /// The subscribed event did happen
    Ok(A),
    /// The subscribed event gave an error
    Err(E),
}

impl<A, E> SchedResult<A, E> {
    pub fn is_not_none(&self) -> bool {
        match self {
            Self::None => false,
            _ => true,
        }
    }
}

pub struct RwSubscription {
    pub handle: EntryHandle,
    status: SchedResult<(Filesize, Eventrwflags), Error>,
}

impl RwSubscription {
    pub fn new(handle: EntryHandle) -> Self {
        Self {
            handle,
            status: SchedResult::None,
        }
    }
    pub fn complete(&mut self, size: Filesize, flags: Eventrwflags) {
        self.status = SchedResult::Ok((size, flags))
    }
    pub fn error(&mut self, error: Error) {
        self.status = SchedResult::Err(error)
    }
    pub fn result(self) -> SchedResult<(Filesize, Eventrwflags), Error> {
        self.status
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
    pub fn result(&self) -> SchedResult<(), Error> {
        if self.deadline.duration_since(SystemTime::now()).is_ok() {
            SchedResult::Ok(())
        } else {
            SchedResult::None
        }
    }
}

pub enum Subscription {
    Read(RwSubscription),
    Write(RwSubscription),
    Timer(TimerSubscription),
}

impl Subscription {
    pub fn earliest_deadline<'a>(
        subs: impl Iterator<Item = &'a Subscription> + 'a,
    ) -> Option<SystemTime> {
        subs.filter_map(|s| match s {
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
    Read(SchedResult<(Filesize, Eventrwflags), Error>),
    Write(SchedResult<(Filesize, Eventrwflags), Error>),
    Timer(SchedResult<(), Error>),
}

impl SubscriptionResult {
    pub fn is_not_none(&self) -> bool {
        match self {
            SubscriptionResult::Read(r) => r.is_not_none(),
            SubscriptionResult::Write(r) => r.is_not_none(),
            SubscriptionResult::Timer(r) => r.is_not_none(),
        }
    }
}

impl From<Subscription> for SubscriptionResult {
    fn from(s: Subscription) -> SubscriptionResult {
        match s {
            Subscription::Read(s) => SubscriptionResult::Read(s.result()),
            Subscription::Write(s) => SubscriptionResult::Write(s.result()),
            Subscription::Timer(s) => SubscriptionResult::Timer(s.result()),
        }
    }
}

pub struct Poll<'a> {
    ctx: &'a WasiCtx,
    subscriptions: Vec<(Subscription, Userdata)>,
}

impl<'a> Poll<'a> {
    pub fn new(ctx: &'a WasiCtx) -> Self {
        Poll {
            ctx,
            subscriptions: Vec::new(),
        }
    }

    pub fn subscribe_timer(
        &mut self,
        at: Timestamp,
        ud: Userdata,
        is_absolute: bool,
    ) -> Result<(), Error> {
        self.subscriptions.push((
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
        self.subscriptions.push((
            Subscription::Timer(TimerSubscription::from_relative(at)?),
            ud,
        ));
        Ok(())
    }

    pub fn subscribe_absolute_time(&mut self, at: Timestamp, ud: Userdata) -> Result<(), Error> {
        self.subscriptions.push((
            Subscription::Timer(TimerSubscription::from_absolute(at)?),
            ud,
        ));
        Ok(())
    }

    pub fn subscribe_fd_read(&mut self, fd: Fd, ud: Userdata) -> Result<(), Error> {
        let entry = self.ctx.get_entry(fd)?;
        let required_rights = HandleRights::from_base(Rights::FD_READ | Rights::POLL_FD_READWRITE);
        let handle = entry.as_handle(&required_rights)?;
        self.subscriptions
            .push((Subscription::Read(RwSubscription::new(handle)), ud));
        Ok(())
    }

    pub fn subscribe_fd_write(&mut self, fd: Fd, ud: Userdata) -> Result<(), Error> {
        let entry = self.ctx.get_entry(fd)?;
        let required_rights = HandleRights::from_base(Rights::FD_WRITE | Rights::POLL_FD_READWRITE);
        let handle = entry.as_handle(&required_rights)?;
        self.subscriptions
            .push((Subscription::Write(RwSubscription::new(handle)), ud));
        Ok(())
    }

    pub fn subsciptions(&mut self) -> Vec<&mut Subscription> {
        self.subscriptions.iter_mut().map(|(s, _ud)| s).collect()
    }

    pub fn earliest_deadline(&self) -> Option<SystemTime> {
        Subscription::earliest_deadline(self.subscriptions.iter().map(|(s, _ud)| s))
    }

    pub fn results(self) -> Vec<(SubscriptionResult, Userdata)> {
        self.subscriptions
            .into_iter()
            .map(|(s, ud)| (SubscriptionResult::from(s), ud))
            .filter(|(r, ud)| r.is_not_none())
            .collect()
    }
}
