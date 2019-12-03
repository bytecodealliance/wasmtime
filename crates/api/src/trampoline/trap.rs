use std::cell::Cell;

use crate::r#ref::HostRef;
use crate::Trap;

thread_local! {
    static RECORDED_API_TRAP: Cell<Option<HostRef<Trap>>> = Cell::new(None);
}

pub fn record_api_trap(trap: HostRef<Trap>) {
    RECORDED_API_TRAP.with(|data| {
        let trap = Cell::new(Some(trap));
        data.swap(&trap);
        assert!(
            trap.take().is_none(),
            "Only one API trap per thread can be recorded at a moment!"
        );
    });
}

pub fn take_api_trap() -> Option<HostRef<Trap>> {
    RECORDED_API_TRAP.with(|data| data.take())
}
