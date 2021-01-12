use parking_lot::Mutex;
use std::collections::HashMap;

use crate::error::*;

pub type Handle = u32;

struct HandlesManagerInner<HandleType: Clone + Sync> {
    last_handle: Handle,
    map: HashMap<Handle, HandleType>,
    type_id: u8,
}

pub struct HandlesManager<HandleType: Clone + Send + Sync> {
    inner: Mutex<HandlesManagerInner<HandleType>>,
}

impl<HandleType: Clone + Send + Sync> HandlesManager<HandleType> {
    pub fn new(handle_type: u8) -> Self {
        HandlesManager {
            inner: Mutex::new(HandlesManagerInner::new(handle_type)),
        }
    }

    pub fn close(&self, handle: Handle) -> Result<(), CryptoError> {
        self.inner.lock().close(handle)
    }

    pub fn register(&self, op: HandleType) -> Result<Handle, CryptoError> {
        self.inner.lock().register(op)
    }

    pub fn get(&self, handle: Handle) -> Result<HandleType, CryptoError> {
        self.inner.lock().get(handle).map(|x| x.clone())
    }
}

impl<HandleType: Clone + Send + Sync> HandlesManagerInner<HandleType> {
    pub fn new(type_id: u8) -> Self {
        HandlesManagerInner {
            last_handle: (type_id as Handle).rotate_right(8),
            map: HashMap::new(),
            type_id,
        }
    }

    pub fn close(&mut self, handle: Handle) -> Result<(), CryptoError> {
        self.map.remove(&handle).ok_or(CryptoError::Closed)?;
        Ok(())
    }

    fn next_handle(&self, handle: Handle) -> Handle {
        ((handle.wrapping_add(1) << 8) | (self.type_id as Handle)).rotate_right(8)
    }

    pub fn register(&mut self, op: HandleType) -> Result<Handle, CryptoError> {
        let mut handle = self.next_handle(self.last_handle);
        loop {
            if !self.map.contains_key(&handle) {
                break;
            }
            ensure!(handle != self.last_handle, CryptoError::TooManyHandles);
            handle = self.next_handle(self.last_handle);
        }
        self.last_handle = handle;
        ensure!(
            self.map.insert(handle, op).is_none(),
            CryptoError::InternalError
        );
        Ok(handle)
    }

    pub fn get(&mut self, handle: Handle) -> Result<&HandleType, CryptoError> {
        let op = self.map.get(&handle).ok_or(CryptoError::InvalidHandle)?;
        Ok(op)
    }
}
