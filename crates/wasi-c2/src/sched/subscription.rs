use crate::file::WasiFile;
use crate::Error;
use bitflags::bitflags;
use cap_std::time::{SystemClock, SystemTime};
use std::cell::{Cell, Ref};

bitflags! {
    pub struct RwEventFlags: u32 {
        const HANGUP = 0b1;
    }
}

pub struct RwSubscription<'a> {
    file: Ref<'a, dyn WasiFile>,
    status: Cell<Option<Result<(u64, RwEventFlags), Error>>>,
}

impl<'a> RwSubscription<'a> {
    pub fn new(file: Ref<'a, dyn WasiFile>) -> Self {
        Self {
            file,
            status: Cell::new(None),
        }
    }
    pub fn complete(&self, size: u64, flags: RwEventFlags) {
        self.status.set(Some(Ok((size, flags))))
    }
    pub fn error(&self, error: Error) {
        self.status.set(Some(Err(error)))
    }
    pub fn result(self) -> Option<Result<(u64, RwEventFlags), Error>> {
        self.status.into_inner()
    }
}

pub struct TimerSubscription {
    pub deadline: SystemTime,
}

impl TimerSubscription {
    pub fn result(&self, clock: &SystemClock) -> Option<Result<(), Error>> {
        if self.deadline.duration_since(clock.now()).is_ok() {
            Some(Ok(()))
        } else {
            None
        }
    }
}

pub enum Subscription<'a> {
    Read(RwSubscription<'a>),
    Write(RwSubscription<'a>),
    Timer(TimerSubscription),
}

pub struct SubscriptionSet<'a> {
    pub subs: Vec<&'a Subscription<'a>>,
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
    Read(Result<(u64, RwEventFlags), Error>),
    Write(Result<(u64, RwEventFlags), Error>),
    Timer(Result<(), Error>),
}

impl SubscriptionResult {
    pub fn from_subscription(s: Subscription, clock: &SystemClock) -> Option<SubscriptionResult> {
        match s {
            Subscription::Read(s) => s.result().map(SubscriptionResult::Read),
            Subscription::Write(s) => s.result().map(SubscriptionResult::Write),
            Subscription::Timer(s) => s.result(clock).map(SubscriptionResult::Timer),
        }
    }
}
