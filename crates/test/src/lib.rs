use proptest::prelude::*;
use wiggle_runtime::GuestMemory;

#[repr(align(4096))]
pub struct HostMemory {
    buffer: [u8; 4096],
}
impl HostMemory {
    pub fn new() -> Self {
        HostMemory { buffer: [0; 4096] }
    }

    pub fn guest_memory<'a>(&'a mut self) -> GuestMemory<'a> {
        GuestMemory::new(self.buffer.as_mut_ptr(), self.buffer.len() as u32)
    }

    pub fn mem_area_strat(align: u32) -> BoxedStrategy<MemArea> {
        prop::num::u32::ANY
            .prop_filter_map("needs to fit in memory", move |p| {
                let p_aligned = p - (p % align); // Align according to argument
                let ptr = p_aligned % 4096; // Put inside memory
                if ptr + align < 4096 {
                    Some(MemArea { ptr, len: align })
                } else {
                    None
                }
            })
            .boxed()
    }
}

#[derive(Debug)]
pub struct MemArea {
    pub ptr: u32,
    pub len: u32,
}

impl MemArea {
    // This code is a whole lot like the Region::overlaps func thats at the core of the code under
    // test.
    // So, I implemented this one with std::ops::Range so it is less likely I wrote the same bug in two
    // places.
    pub fn overlapping(&self, b: &Self) -> bool {
        // a_range is all elems in A
        let a_range = std::ops::Range {
            start: self.ptr,
            end: self.ptr + self.len, // std::ops::Range is open from the right
        };
        // b_range is all elems in B
        let b_range = std::ops::Range {
            start: b.ptr,
            end: b.ptr + b.len,
        };
        // No element in B is contained in A:
        for b_elem in b_range.clone() {
            if a_range.contains(&b_elem) {
                return true;
            }
        }
        // No element in A is contained in B:
        for a_elem in a_range {
            if b_range.contains(&a_elem) {
                return true;
            }
        }
        return false;
    }
    pub fn non_overlapping_set(areas: &[&Self]) -> bool {
        // A is all areas
        for (i, a) in areas.iter().enumerate() {
            // (A, B) is every pair of areas
            for b in areas[i + 1..].iter() {
                if a.overlapping(b) {
                    return false;
                }
            }
        }
        return true;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn hostmemory_is_aligned() {
        let mut h = HostMemory::new();
        assert_eq!(h.buffer.as_mut_ptr() as usize % 4096, 0);
        let mut h = Box::new(HostMemory::new());
        assert_eq!(h.buffer.as_mut_ptr() as usize % 4096, 0);
    }
}

use wiggle_runtime::GuestError;

pub struct WasiCtx {
    pub guest_errors: Vec<GuestError>,
}

impl WasiCtx {
    pub fn new() -> Self {
        Self {
            guest_errors: vec![],
        }
    }
}

// Errno is used as a first return value in the functions above, therefore
// it must implement GuestErrorType with type Context = WasiCtx.
// The context type should let you do logging or debugging or whatever you need
// with these errors. We just push them to vecs.
#[macro_export]
macro_rules! impl_errno {
    ( $errno:ty ) => {
        impl wiggle_runtime::GuestErrorType for $errno {
            type Context = WasiCtx;
            fn success() -> $errno {
                <$errno>::Ok
            }
            fn from_error(e: GuestError, ctx: &mut WasiCtx) -> $errno {
                eprintln!("GUEST ERROR: {:?}", e);
                ctx.guest_errors.push(e);
                types::Errno::InvalidArg
            }
        }
    };
}
