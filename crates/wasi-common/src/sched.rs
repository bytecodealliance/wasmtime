use crate::clocks::WasiMonotonicClock;
use crate::table::Table;
use crate::{Error, ErrorExt};
use cap_std::time::Instant;
use std::collections::HashSet;
pub mod subscription;
pub use cap_std::time::Duration;

use subscription::{MonotonicClockSubscription, RwSubscription, Subscription, SubscriptionResult};

#[wiggle::async_trait]
pub trait WasiSched {
    async fn poll_oneoff<'a>(&self, poll: &Poll<'a>) -> Result<(), Error>;
    async fn sched_yield(&self) -> Result<(), Error>;
    async fn sleep(&self, duration: Duration) -> Result<(), Error>;
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
    table: &'a Table,
    fds: HashSet<u32>,
    subs: Vec<(Subscription<'a>, Userdata)>,
}

impl<'a> Poll<'a> {
    pub fn new(table: &'a Table) -> Self {
        Self {
            table,
            fds: HashSet::new(),
            subs: Vec::new(),
        }
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
    pub fn subscribe_read(&mut self, fd: u32, ud: Userdata) -> Result<(), Error> {
        if self.fds.contains(&fd) {
            return Err(
                Error::invalid_argument().context("Fd can be subscribed to at most once per poll")
            );
        } else {
            self.fds.insert(fd);
        }
        self.subs
            .push((Subscription::Read(RwSubscription::new(self.table, fd)?), ud));
        Ok(())
    }
    pub fn subscribe_write(&mut self, fd: u32, ud: Userdata) -> Result<(), Error> {
        if self.fds.contains(&fd) {
            return Err(
                Error::invalid_argument().context("Fd can be subscribed to at most once per poll")
            );
        } else {
            self.fds.insert(fd);
        }
        self.subs.push((
            Subscription::Write(RwSubscription::new(self.table, fd)?),
            ud,
        ));
        Ok(())
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
