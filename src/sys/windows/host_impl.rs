//! WASI host types specific to Windows host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
use crate::host;

use std::marker::PhantomData;
use std::slice;
use winapi::shared::{ntdef, ws2def};

// these will be obsolete once https://github.com/rust-lang/rust/pull/60334
// lands in stable
pub struct IoVec<'a> {
    vec: ws2def::WSABUF,
    _p: PhantomData<&'a [u8]>,
}

pub struct IoVecMut<'a> {
    vec: ws2def::WSABUF,
    _p: PhantomData<&'a mut [u8]>,
}

impl<'a> IoVec<'a> {
    #[inline]
    pub fn new(buf: &'a [u8]) -> Self {
        assert!(buf.len() <= ntdef::ULONG::max_value() as usize);
        Self {
            vec: ws2def::WSABUF {
                len: buf.len() as ntdef::ULONG,
                buf: buf.as_ptr() as *mut u8 as *mut ntdef::CHAR,
            },
            _p: PhantomData,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.vec.buf as *mut u8, self.vec.len as usize) }
    }
}

impl<'a> IoVecMut<'a> {
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> Self {
        assert!(buf.len() <= ntdef::ULONG::max_value() as usize);
        Self {
            vec: ws2def::WSABUF {
                len: buf.len() as ntdef::ULONG,
                buf: buf.as_mut_ptr() as *mut u8 as *mut ntdef::CHAR,
            },
            _p: PhantomData,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.vec.buf as *mut u8, self.vec.len as usize) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.vec.buf as *mut u8, self.vec.len as usize) }
    }
}

pub unsafe fn ciovec_to_win<'a>(ciovec: &'a host::__wasi_ciovec_t) -> IoVec<'a> {
    let slice = slice::from_raw_parts(ciovec.buf as *const u8, ciovec.buf_len);
    IoVec::new(slice)
}

pub unsafe fn ciovec_to_win_mut<'a>(ciovec: &'a mut host::__wasi_ciovec_t) -> IoVecMut<'a> {
    let slice = slice::from_raw_parts_mut(ciovec.buf as *mut u8, ciovec.buf_len);
    IoVecMut::new(slice)
}

pub unsafe fn iovec_to_win<'a>(iovec: &'a host::__wasi_iovec_t) -> IoVec<'a> {
    let slice = slice::from_raw_parts(iovec.buf as *const u8, iovec.buf_len);
    IoVec::new(slice)
}

pub unsafe fn iovec_to_win_mut<'a>(iovec: &'a mut host::__wasi_iovec_t) -> IoVecMut<'a> {
    let slice = slice::from_raw_parts_mut(iovec.buf as *mut u8, iovec.buf_len);
    IoVecMut::new(slice)
}
