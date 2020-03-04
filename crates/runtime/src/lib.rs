use std::cell::Cell;
use std::fmt;
use std::marker;
use std::slice;
use std::str;

mod error;
mod guest_type;
mod region;
pub use error::GuestError;
pub use guest_type::{GuestErrorType, GuestType};
pub use region::Region;

pub unsafe trait GuestMemory {
    fn base(&self) -> (*mut u8, u32);

    fn validate_size_align(
        &self,
        offset: u32,
        align: usize,
        len: u32,
    ) -> Result<*mut u8, GuestError> {
        let (base_ptr, base_len) = self.base();
        let region = Region { start: offset, len };

        // Figure out our pointer to the start of memory
        let start = match (base_ptr as usize).checked_add(offset as usize) {
            Some(ptr) => ptr,
            None => return Err(GuestError::PtrOutOfBounds(region)),
        };
        // and use that to figure out the end pointer
        let end = match start.checked_add(len as usize) {
            Some(ptr) => ptr,
            None => return Err(GuestError::PtrOutOfBounds(region)),
        };
        // and then verify that our end doesn't reach past the end of our memory
        if end > (base_ptr as usize) + (base_len as usize) {
            return Err(GuestError::PtrOutOfBounds(region));
        }
        // and finally verify that the alignment is correct
        if start % align != 0 {
            return Err(GuestError::PtrNotAligned(region, align as u32));
        }
        Ok(start as *mut u8)
    }

    fn ptr<'a, T>(&'a self, offset: T::Pointer) -> GuestPtr<'a, T>
    where
        Self: Sized,
        T: ?Sized + Pointee,
    {
        GuestPtr::new(self, offset)
    }
}

unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a T {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
}

unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a mut T {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
}

pub struct GuestPtr<'a, T: ?Sized + Pointee> {
    mem: &'a (dyn GuestMemory + 'a),
    pointer: T::Pointer,
    _marker: marker::PhantomData<&'a Cell<T>>,
}

impl<'a, T: ?Sized + Pointee> GuestPtr<'a, T> {
    pub fn new(mem: &'a (dyn GuestMemory + 'a), pointer: T::Pointer) -> GuestPtr<'_, T> {
        GuestPtr {
            mem,
            pointer,
            _marker: marker::PhantomData,
        }
    }

    pub fn offset(&self) -> T::Pointer {
        self.pointer
    }

    pub fn mem(&self) -> &'a (dyn GuestMemory + 'a) {
        self.mem
    }

    pub fn cast<U>(&self) -> GuestPtr<'a, U>
    where
        T: Pointee<Pointer = u32>,
    {
        GuestPtr::new(self.mem, self.pointer)
    }

    pub fn read(&self) -> Result<T, GuestError>
    where
        T: GuestType<'a>,
    {
        T::read(self)
    }

    pub fn write(&self, val: T) -> Result<(), GuestError>
    where
        T: GuestType<'a>,
    {
        T::write(self, val)
    }

    pub fn add(&self, amt: u32) -> Result<GuestPtr<'a, T>, GuestError>
    where
        T: GuestType<'a> + Pointee<Pointer = u32>,
    {
        let offset = amt
            .checked_mul(T::guest_size())
            .and_then(|o| self.pointer.checked_add(o));
        let offset = match offset {
            Some(o) => o,
            None => return Err(GuestError::InvalidFlagValue("")),
        };
        Ok(GuestPtr::new(self.mem, offset))
    }
}

impl<'a, T> GuestPtr<'a, [T]> {
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    pub fn iter<'b>(
        &'b self,
    ) -> impl ExactSizeIterator<Item = Result<GuestPtr<'a, T>, GuestError>> + 'b
    where
        T: GuestType<'a>,
    {
        let base = GuestPtr::new(self.mem, self.offset_base());
        (0..self.len()).map(move |i| base.add(i))
    }
}

impl<'a> GuestPtr<'a, str> {
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    pub fn as_bytes(&self) -> GuestPtr<'a, [u8]> {
        GuestPtr::new(self.mem, self.pointer)
    }

    pub fn as_raw(&self) -> Result<*mut str, GuestError> {
        let ptr = self
            .mem
            .validate_size_align(self.pointer.0, 1, self.pointer.1)?;

        // TODO: doc unsafety here
        unsafe {
            let s = slice::from_raw_parts_mut(ptr, self.pointer.1 as usize);
            match str::from_utf8_mut(s) {
                Ok(s) => Ok(s),
                Err(e) => Err(GuestError::InvalidUtf8(e)),
            }
        }
    }
}

impl<T: ?Sized + Pointee> Clone for GuestPtr<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Pointee> Copy for GuestPtr<'_, T> {}

impl<T: ?Sized + Pointee> fmt::Debug for GuestPtr<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::debug(self.pointer, f)
    }
}

mod private {
    pub trait Sealed {}
    impl<T> Sealed for T {}
    impl<T> Sealed for [T] {}
    impl Sealed for str {}
}

pub trait Pointee: private::Sealed {
    #[doc(hidden)]
    type Pointer: Copy;
    #[doc(hidden)]
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result;
}

impl<T> Pointee for T {
    type Pointer = u32;
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}", pointer)
    }
}

impl<T> Pointee for [T] {
    type Pointer = (u32, u32);
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}/{}", pointer.0, pointer.1)
    }
}

impl Pointee for str {
    type Pointer = (u32, u32);
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        <[u8]>::debug(pointer, f)
    }
}
