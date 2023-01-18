use crate::clocks::WasiMonotonicClock;
use crate::stream::WasiStream;
use crate::Error;
use bitflags::bitflags;

bitflags! {
    pub struct RwEventFlags: u32 {
        const HANGUP = 0b1;
    }
}

pub struct RwSubscription<'a> {
    pub stream: &'a dyn WasiStream,
    status: Option<Result<RwEventFlags, Error>>,
}

impl<'a> RwSubscription<'a> {
    pub fn new(stream: &'a dyn WasiStream) -> Self {
        Self {
            stream,
            status: None,
        }
    }
    pub fn complete(&mut self, flags: RwEventFlags) {
        self.status = Some(Ok(flags))
    }
    pub fn error(&mut self, error: Error) {
        self.status = Some(Err(error))
    }
    pub fn result(&mut self) -> Option<Result<RwEventFlags, Error>> {
        self.status.take()
    }
    pub fn is_complete(&self) -> bool {
        self.status.is_some()
    }
}

pub struct MonotonicClockSubscription<'a> {
    pub clock: &'a dyn WasiMonotonicClock,
    pub deadline: u64,
}

impl<'a> MonotonicClockSubscription<'a> {
    pub fn now(&self) -> u64 {
        self.clock.now()
    }
    pub fn duration_until(&self) -> Option<u64> {
        self.deadline.checked_sub(self.now())
    }
    pub fn result(&self) -> Option<Result<(), Error>> {
        if self.now().checked_sub(self.deadline).is_some() {
            Some(Ok(()))
        } else {
            None
        }
    }
}

pub enum Subscription<'a> {
    ReadWrite(RwSubscription<'a>, RwSubscriptionKind),
    MonotonicClock(MonotonicClockSubscription<'a>),
}

#[derive(Copy, Clone, Debug)]
pub enum RwSubscriptionKind {
    Read,
    Write,
}

#[derive(Debug)]
pub enum SubscriptionResult {
    ReadWrite(Result<RwEventFlags, Error>, RwSubscriptionKind),
    MonotonicClock(Result<(), Error>),
}

impl SubscriptionResult {
    pub fn from_subscription(s: Subscription) -> Option<SubscriptionResult> {
        match s {
            Subscription::ReadWrite(mut s, kind) => s
                .result()
                .map(|sub| SubscriptionResult::ReadWrite(sub, kind)),
            Subscription::MonotonicClock(s) => s.result().map(SubscriptionResult::MonotonicClock),
        }
    }
}
