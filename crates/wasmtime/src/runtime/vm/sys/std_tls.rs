use std::cell::Cell;

std::thread_local!(static TLS: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) });

#[inline]
pub fn tls_get() -> *mut u8 {
    TLS.with(|p| p.get())
}

#[inline]
pub fn tls_set(ptr: *mut u8) {
    TLS.with(|p| p.set(ptr));
}

#[cfg(feature = "component-model-async")]
std::thread_local!(static COMPONENT_ASYNC_TLS: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) });

#[inline]
#[cfg(feature = "component-model-async")]
pub fn component_async_tls_get() -> *mut u8 {
    COMPONENT_ASYNC_TLS.with(|p| p.get())
}

#[inline]
#[cfg(feature = "component-model-async")]
pub fn component_async_tls_set(ptr: *mut u8) {
    COMPONENT_ASYNC_TLS.with(|p| p.set(ptr));
}
