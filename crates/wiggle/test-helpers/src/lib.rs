use proptest::prelude::*;
use std::marker;
use wiggle::GuestMemory;

#[derive(Debug, Clone)]
pub struct MemAreas(Vec<MemArea>);
impl MemAreas {
    pub fn new() -> Self {
        MemAreas(Vec::new())
    }
    pub fn insert(&mut self, a: MemArea) {
        // Find if `a` is already in the vector
        match self.0.binary_search(&a) {
            // It is present - insert it next to existing one
            Ok(loc) => self.0.insert(loc, a),
            // It is not present - heres where to insert it
            Err(loc) => self.0.insert(loc, a),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &MemArea> {
        self.0.iter()
    }
}

impl<R> From<R> for MemAreas
where
    R: AsRef<[MemArea]>,
{
    fn from(ms: R) -> MemAreas {
        let mut out = MemAreas::new();
        for m in ms.as_ref().into_iter() {
            out.insert(*m);
        }
        out
    }
}

impl Into<Vec<MemArea>> for MemAreas {
    fn into(self) -> Vec<MemArea> {
        self.0.clone()
    }
}

#[repr(align(4096))]
struct HostBuffer {
    cell: [u8; 4096],
}

unsafe impl Send for HostBuffer {}
unsafe impl Sync for HostBuffer {}

pub struct HostMemory {
    buffer: HostBuffer,
}
impl HostMemory {
    pub fn new() -> Self {
        HostMemory {
            buffer: HostBuffer { cell: [0; 4096] },
        }
    }

    pub fn guest_memory(&mut self) -> GuestMemory<'_> {
        GuestMemory::Unshared(&mut self.buffer.cell)
    }

    pub fn base(&self) -> *const u8 {
        self.buffer.cell.as_ptr()
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

    /// Takes a sorted list or memareas, and gives a sorted list of memareas covering
    /// the parts of memory not covered by the previous
    pub fn invert(regions: &MemAreas) -> MemAreas {
        let mut out = MemAreas::new();
        let mut start = 0;
        for r in regions.iter() {
            let len = r.ptr - start;
            if len > 0 {
                out.insert(MemArea {
                    ptr: start,
                    len: r.ptr - start,
                });
            }
            start = r.ptr + r.len;
        }
        if start < 4096 {
            out.insert(MemArea {
                ptr: start,
                len: 4096 - start,
            });
        }
        out
    }

    pub fn byte_slice_strat(size: u32, align: u32, exclude: &MemAreas) -> BoxedStrategy<MemArea> {
        let available: Vec<MemArea> = Self::invert(exclude)
            .iter()
            .flat_map(|a| a.inside(size))
            .filter(|a| a.ptr % align == 0)
            .collect();

        Just(available)
            .prop_filter("available memory for allocation", |a| !a.is_empty())
            .prop_flat_map(|a| prop::sample::select(a))
            .boxed()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemArea {
    pub ptr: u32,
    pub len: u32,
}

impl MemArea {
    // This code is a whole lot like the Region::overlaps func that's at the core of the code under
    // test.
    // So, I implemented this one with std::ops::Range so it is less likely I wrote the same bug in two
    // places.
    pub fn overlapping(&self, b: Self) -> bool {
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
    pub fn non_overlapping_set<M>(areas: M) -> bool
    where
        M: Into<MemAreas>,
    {
        let areas = areas.into();
        for (aix, a) in areas.iter().enumerate() {
            for (bix, b) in areas.iter().enumerate() {
                if aix != bix {
                    // (A, B) is every pairing of areas
                    if a.overlapping(*b) {
                        return false;
                    }
                }
            }
        }
        return true;
    }

    /// Enumerate all memareas of size `len` inside a given area
    fn inside(&self, len: u32) -> impl Iterator<Item = MemArea> + use<'_> {
        let end: i64 = self.len as i64 - len as i64;
        let start = self.ptr;
        (0..end).into_iter().map(move |v| MemArea {
            ptr: start + v as u32,
            len,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hostmemory_is_aligned() {
        let h = HostMemory::new();
        assert_eq!(h.base() as usize % 4096, 0);
        let h = Box::new(h);
        assert_eq!(h.base() as usize % 4096, 0);
    }

    #[test]
    fn invert() {
        fn invert_equality(input: &[MemArea], expected: &[MemArea]) {
            let input: MemAreas = input.into();
            let inverted: Vec<MemArea> = HostMemory::invert(&input).into();
            assert_eq!(expected, inverted.as_slice());
        }

        invert_equality(&[], &[MemArea { ptr: 0, len: 4096 }]);
        invert_equality(
            &[MemArea { ptr: 0, len: 1 }],
            &[MemArea { ptr: 1, len: 4095 }],
        );

        invert_equality(
            &[MemArea { ptr: 1, len: 1 }],
            &[MemArea { ptr: 0, len: 1 }, MemArea { ptr: 2, len: 4094 }],
        );

        invert_equality(
            &[MemArea { ptr: 1, len: 4095 }],
            &[MemArea { ptr: 0, len: 1 }],
        );

        invert_equality(
            &[MemArea { ptr: 0, len: 1 }, MemArea { ptr: 1, len: 4095 }],
            &[],
        );

        invert_equality(
            &[MemArea { ptr: 1, len: 2 }, MemArea { ptr: 4, len: 1 }],
            &[
                MemArea { ptr: 0, len: 1 },
                MemArea { ptr: 3, len: 1 },
                MemArea { ptr: 5, len: 4091 },
            ],
        );
    }

    fn set_of_slices_strat(
        s1: u32,
        s2: u32,
        s3: u32,
    ) -> BoxedStrategy<(MemArea, MemArea, MemArea)> {
        HostMemory::byte_slice_strat(s1, 1, &MemAreas::new())
            .prop_flat_map(move |a1| {
                (
                    Just(a1),
                    HostMemory::byte_slice_strat(s2, 1, &MemAreas::from(&[a1])),
                )
            })
            .prop_flat_map(move |(a1, a2)| {
                (
                    Just(a1),
                    Just(a2),
                    HostMemory::byte_slice_strat(s3, 1, &MemAreas::from(&[a1, a2])),
                )
            })
            .boxed()
    }

    #[test]
    fn trivial_inside() {
        let a = MemArea { ptr: 24, len: 4072 };
        let interior = a.inside(24).collect::<Vec<_>>();

        assert!(interior.len() > 0);
    }

    proptest! {
        #[test]
        // For some random region of decent size
        fn inside(r in HostMemory::mem_area_strat(123)) {
            let set_of_r = MemAreas::from(&[r]);
            // All regions outside of r:
            let exterior = HostMemory::invert(&set_of_r);
            // All regions inside of r:
            let interior = r.inside(22);
            for i in interior {
                // i overlaps with r:
                assert!(r.overlapping(i));
                // i is inside r:
                assert!(i.ptr >= r.ptr);
                assert!(r.ptr + r.len >= i.ptr + i.len);
                // the set of exterior and i is non-overlapping
                let mut all = exterior.clone();
                all.insert(i);
                assert!(MemArea::non_overlapping_set(all));
            }
        }

        #[test]
        fn byte_slices((s1, s2, s3) in set_of_slices_strat(12, 34, 56)) {
            let all = MemAreas::from(&[s1, s2, s3]);
            assert!(MemArea::non_overlapping_set(all));
        }
    }
}

use std::cell::RefCell;
use wiggle::GuestError;

// In lucet, our Ctx struct needs a lifetime, so we're using one
// on the test as well.
pub struct WasiCtx<'a> {
    pub guest_errors: RefCell<Vec<GuestError>>,
    pub log: RefCell<Vec<String>>,
    lifetime: marker::PhantomData<&'a ()>,
}

impl<'a> WasiCtx<'a> {
    pub fn new() -> Self {
        Self {
            guest_errors: RefCell::new(vec![]),
            log: RefCell::new(vec![]),
            lifetime: marker::PhantomData,
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
        impl wiggle::GuestErrorType for $errno {
            fn success() -> $errno {
                <$errno>::Ok
            }
        }
    };
}
