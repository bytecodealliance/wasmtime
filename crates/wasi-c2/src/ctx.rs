use crate::table::Table;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

pub struct WasiCtx {
    table: Rc<RefCell<Table>>,
}

impl WasiCtx {
    pub fn new() -> Self {
        WasiCtx {
            table: Rc::new(RefCell::new(Table::new())),
        }
    }

    pub fn table(&self) -> RefMut<Table> {
        self.table.borrow_mut()
    }
}

pub trait WasiDir {}

pub(crate) struct DirEntry {
    pub(crate) flags: u32,
    pub(crate) dir: Box<dyn WasiDir>,
}
