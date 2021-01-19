use crate::clocks::WasiMonotonicClock;
use crate::file::WasiFile;
use crate::Error;
use bitflags::bitflags;
use cap_std::time::{Duration, Instant};
use std::cell::{Cell, Ref};

bitflags! {
    pub struct RwEventFlags: u32 {
        const HANGUP = 0b1;
    }
}

pub struct RwSubscription<'a> {
    pub file: Ref<'a, dyn WasiFile>,
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

pub struct MonotonicClockSubscription<'a> {
    pub clock: &'a dyn WasiMonotonicClock,
    pub deadline: Instant,
    pub precision: Duration,
}

impl<'a> MonotonicClockSubscription<'a> {
    pub fn now(&self) -> Instant {
        self.clock.now(self.precision)
    }
    pub fn result(&self) -> Option<Result<(), Error>> {
        if self.now().checked_duration_since(self.deadline).is_some() {
            Some(Ok(()))
        } else {
            None
        }
    }
}

pub enum Subscription<'a> {
    Read(RwSubscription<'a>),
    Write(RwSubscription<'a>),
    MonotonicClock(MonotonicClockSubscription<'a>),
}

pub enum SubscriptionResult {
    Read(Result<(u64, RwEventFlags), Error>),
    Write(Result<(u64, RwEventFlags), Error>),
    MonotonicClock(Result<(), Error>),
}

impl SubscriptionResult {
    pub fn from_subscription(s: Subscription) -> Option<SubscriptionResult> {
        match s {
            Subscription::Read(s) => s.result().map(SubscriptionResult::Read),
            Subscription::Write(s) => s.result().map(SubscriptionResult::Write),
            Subscription::MonotonicClock(s) => s.result().map(SubscriptionResult::MonotonicClock),
        }
    }
}
