use crate::clocks::WasiMonotonicClock;
use crate::file::WasiFile;
use crate::Error;
use cap_std::time::{Duration, Instant};
use std::cell::Ref;
pub mod subscription;

use subscription::{MonotonicClockSubscription, RwSubscription, Subscription, SubscriptionResult};

pub trait WasiSched {
    fn poll_oneoff(&self, poll: &Poll) -> Result<(), Error>;
    fn sched_yield(&self) -> Result<(), Error>;
}

pub struct Userdata(u64);
impl From<u64> for Userdata {
    fn from(u: u64) -> Userdata {
        Userdata(u)
    }
}

impl From<Userdata> for u64 {
    fn from(u: Userdata) -> u64 {
        u.0
    }
}

pub struct Poll<'a> {
    subs: Vec<(Subscription<'a>, Userdata)>,
}

impl<'a> Poll<'a> {
    pub fn new() -> Self {
        Self { subs: Vec::new() }
    }
    pub fn subscribe_monotonic_clock(
        &mut self,
        clock: &'a dyn WasiMonotonicClock,
        deadline: Instant,
        precision: Duration,
        ud: Userdata,
    ) {
        self.subs.push((
            Subscription::MonotonicClock(MonotonicClockSubscription {
                clock,
                deadline,
                precision,
            }),
            ud,
        ));
    }
    pub fn subscribe_read(&mut self, file: Ref<'a, dyn WasiFile>, ud: Userdata) {
        self.subs
            .push((Subscription::Read(RwSubscription::new(file)), ud));
    }
    pub fn subscribe_write(&mut self, file: Ref<'a, dyn WasiFile>, ud: Userdata) {
        self.subs
            .push((Subscription::Write(RwSubscription::new(file)), ud));
    }
    pub fn results(self) -> Vec<(SubscriptionResult, Userdata)> {
        self.subs
            .into_iter()
            .filter_map(|(s, ud)| SubscriptionResult::from_subscription(s).map(|r| (r, ud)))
            .collect()
    }
    pub fn is_empty(&self) -> bool {
        self.subs.is_empty()
    }
    pub fn earliest_clock_deadline(&'a self) -> Option<&MonotonicClockSubscription<'a>> {
        self.subs
            .iter()
            .filter_map(|(s, _ud)| match s {
                Subscription::MonotonicClock(t) => Some(t),
                _ => None,
            })
            .min_by(|a, b| a.deadline.cmp(&b.deadline))
    }
    pub fn rw_subscriptions(&'a self) -> impl Iterator<Item = &Subscription<'a>> {
        self.subs.iter().filter_map(|(s, _ud)| match s {
            Subscription::Read { .. } | Subscription::Write { .. } => Some(s),
            _ => None,
        })
    }
}
