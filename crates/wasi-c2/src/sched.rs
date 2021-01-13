use crate::clocks::WasiSystemClock;
use crate::file::WasiFile;
use crate::Error;
use cap_std::time::SystemTime;
use std::cell::Ref;
pub mod subscription;

use subscription::{
    RwSubscription, Subscription, SubscriptionResult, SubscriptionSet, TimerSubscription,
};

pub trait WasiSched {
    fn poll_oneoff<'a>(&self, poll: &mut Poll<'a>) -> Result<(), Error>;
    fn sched_yield(&self) -> Result<(), Error>;
}

#[derive(Default)]
pub struct SyncSched {}

impl WasiSched for SyncSched {
    fn poll_oneoff<'a>(&self, poll: &mut Poll<'a>) -> Result<(), Error> {
        todo!()
    }
    fn sched_yield(&self) -> Result<(), Error> {
        std::thread::yield_now();
        Ok(())
    }
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
    pub fn subscribe_timer(&mut self, deadline: SystemTime, ud: Userdata) {
        self.subs
            .push((Subscription::Timer(TimerSubscription { deadline }), ud));
    }
    pub fn subscribe_read(&mut self, file: Ref<'a, dyn WasiFile>, ud: Userdata) {
        self.subs
            .push((Subscription::Read(RwSubscription::new(file)), ud));
    }
    pub fn subscribe_write(&mut self, file: Ref<'a, dyn WasiFile>, ud: Userdata) {
        self.subs
            .push((Subscription::Read(RwSubscription::new(file)), ud));
    }
    pub fn results(self, clock: &dyn WasiSystemClock) -> Vec<(SubscriptionResult, Userdata)> {
        self.subs
            .into_iter()
            .filter_map(|(s, ud)| SubscriptionResult::from_subscription(s, clock).map(|r| (r, ud)))
            .collect()
    }
    pub(crate) fn subscriptions(&'a mut self) -> SubscriptionSet<'a> {
        SubscriptionSet {
            subs: self.subs.iter().map(|(s, _ud)| s).collect(),
        }
    }
}
