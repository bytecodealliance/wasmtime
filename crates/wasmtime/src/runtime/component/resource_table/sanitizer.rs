use alloc::vec::Vec;
use std::backtrace::Backtrace;
use std::cell::RefCell;

#[derive(Debug)]
pub struct ResourceSanitizer {
    uses: RefCell<Vec<(u32, SanInfo)>>,
}

impl ResourceSanitizer {
    pub fn new() -> Self {
        Self {
            uses: RefCell::new(Vec::new()),
        }
    }
    pub(super) fn log_construction(&self, index: u32, info: SanInfo) {
        self.uses.borrow_mut().push((index, info));
    }
    pub(super) fn log_usage(&self, index: u32, backtrace: Backtrace) {
        let mut uses = self.uses.borrow_mut();
        let (_, info) = uses
            .iter_mut()
            .rev()
            .find(|(ix, _)| index == *ix)
            .expect("used resource present in sanitizer log");
        info.last_used = Some(backtrace);
    }
    pub(super) fn log_delete(&self, index: u32, backtrace: Backtrace) {
        let mut uses = self.uses.borrow_mut();
        let (_, info) = uses
            .iter_mut()
            .rev()
            .find(|(ix, _)| index == *ix)
            .expect("deleted resource present in sanitizer log");
        info.deleted = Some(backtrace);
    }
}

#[derive(Debug)]
pub struct SanInfo {
    type_name: &'static str,
    allocated: std::backtrace::Backtrace,
    last_used: Option<std::backtrace::Backtrace>,
    deleted: Option<std::backtrace::Backtrace>,
}

impl SanInfo {
    pub fn new(type_name: &'static str, allocated: Backtrace) -> Self {
        SanInfo {
            type_name,
            allocated,
            last_used: None,
            deleted: None,
        }
    }
}
