use std::{
    collections::{BTreeMap, HashSet},
    ops,
};

#[derive(Default, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct Ptr(pub usize);

impl Ptr {
    fn offset(self, size: Size) -> Ptr {
        Ptr(self.0 + size.0)
    }
}

#[derive(Default, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct Size(pub usize);

impl ops::Add for Size {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Size(self.0 + other.0)
    }
}

impl ops::Sub for Size {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Size(self.0 - other.0)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Alloc {
    size: Size,
    blocks_by_address: BTreeMap<Ptr, Size>,
    blocks_by_size: BTreeMap<Size, HashSet<Ptr>>,
}

impl Alloc {
    pub fn new(bytes: Size) -> Self {
        let mut out = Self::default();
        out.set_size(bytes);
        out.free(Ptr(0), bytes);
        out
    }

    pub fn set_size(&mut self, size: Size) {
        use std::cmp::Ordering;
        match size.cmp(&self.size) {
            Ordering::Less => self.truncate(size),
            Ordering::Equal | Ordering::Greater => {}
        }

        self.size = size;
    }

    pub fn size(&self) -> Size {
        self.size
    }

    fn truncate(&mut self, size: Size) {
        self.mark_allocated(Ptr(size.0), self.size - size);
    }

    pub fn mark_allocated(&mut self, ptr: Ptr, size: Size) {
        use std::cmp::Ordering;

        let end = ptr.offset(size);

        if let Some((&existing_ptr, &existing_size)) = self
            .blocks_by_address
            .range(..end)
            .last()
            .filter(|(p, s)| **p <= ptr && p.offset(**s) >= end)
        {
            match ptr.cmp(&existing_ptr) {
                Ordering::Less => unreachable!(),
                Ordering::Equal => {
                    self.remove_block(existing_ptr);
                    self.add_block(end, existing_size - size);
                }
                Ordering::Greater => {
                    let existing_end = existing_ptr.offset(existing_size);

                    self.modify_block(existing_ptr, Size(ptr.0 - existing_ptr.0));
                    self.add_block(end, Size(existing_end.0 - end.0));
                }
            }
        }
    }

    pub fn malloc(&mut self, size: Size) -> Option<Ptr> {
        use std::cmp::Ordering;

        let (&existing_size, ptrs) = self.blocks_by_size.range_mut(size..).next()?;

        let ptr = *ptrs.iter().next().expect("Allocator metadata corrupted");

        self.remove_block(ptr);

        match existing_size.cmp(&size) {
            Ordering::Less => unreachable!(),
            Ordering::Equal => {}
            Ordering::Greater => self.add_block(ptr.offset(size), existing_size - size),
        }

        Some(ptr)
    }

    pub fn free(&mut self, ptr: Ptr, size: Size) {
        let prev_block = self
            .blocks_by_address
            .range(..=ptr)
            .last()
            .filter(|(p, s)| p.offset(**s) == ptr)
            .map(|(p, s)| (*p, *s));
        let next_block = self
            .blocks_by_address
            .get_key_value(&ptr.offset(size))
            .map(|(p, s)| (*p, *s));

        match (prev_block, next_block) {
            (Some((prev_ptr, prev_size)), Some((next_ptr, next_size))) => {
                self.remove_block(next_ptr);
                self.modify_block(prev_ptr, prev_size + size + next_size);
            }
            (None, Some((next_ptr, next_size))) => {
                self.remove_block(next_ptr);
                self.add_block(ptr, size + next_size);
            }
            (Some((prev_ptr, prev_size)), None) => {
                self.modify_block(prev_ptr, size + prev_size);
            }
            (None, None) => {
                self.add_block(ptr, size);
            }
        }
    }

    pub fn is_free(&self, ptr: Ptr, size: Size) -> bool {
        self.blocks_by_address
            .range(..=ptr)
            .last()
            .map(|(p, s)| p.offset(*s) >= ptr.offset(size))
            .unwrap_or(false)
    }

    fn modify_block(&mut self, ptr: Ptr, new_size: Size) {
        self.remove_block(ptr);
        self.add_block(ptr, new_size);
    }

    fn add_block(&mut self, ptr: Ptr, size: Size) {
        if size.0 > 0 {
            debug_assert!(self.blocks_by_address.insert(ptr, size).is_none());
            debug_assert!(self.blocks_by_size.entry(size).or_default().insert(ptr));
        }
    }

    fn remove_block(&mut self, ptr: Ptr) {
        use std::collections::btree_map::Entry;

        let size = self
            .blocks_by_address
            .remove(&ptr)
            .expect("Double-free'd block");
        match self.blocks_by_size.entry(size) {
            Entry::Occupied(mut entry) => {
                assert!(entry.get_mut().remove(&ptr), "Allocator metadata corrupted");
                if entry.get().is_empty() {
                    entry.remove_entry();
                }
            }
            Entry::Vacant(_) => panic!("Allocator metadata corrupted"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Alloc, Ptr, Size};

    #[test]
    fn alloc_free() {
        let small = Size(8);
        let mid = Size(16);
        let large = Size(32);
        let mut alloc = Alloc::new(small + mid + large);

        let small = Size(8);
        let mid = Size(16);
        let large = Size(32);
        let a = alloc.malloc(small).unwrap();
        let b = alloc.malloc(mid).unwrap();
        let c = alloc.malloc(large).unwrap();
        assert_eq!(a, Ptr(0));
        assert_eq!(b, Ptr(8));
        assert_eq!(c, Ptr(24));
        assert!(alloc.malloc(small).is_none());
        alloc.free(a, small);
        alloc.free(b, mid);
        let a = alloc.malloc(small).unwrap();
        let b1 = alloc.malloc(small).unwrap();
        let b2 = alloc.malloc(small).unwrap();
        assert!(alloc.malloc(small).is_none());
        assert_eq!(a, Ptr(0));
        assert_eq!(b1, Ptr(8));
        assert_eq!(b2, Ptr(16));
    }

    #[test]
    fn mark_allocated() {
        let small = Size(8);
        let mid = Size(16);
        let large = Size(32);
        let mut alloc = Alloc::new(small + mid + large);

        let small = Size(8);
        let mid = Size(16);
        let large = Size(32);
        let a = Ptr(0);
        let b = Ptr(8);
        alloc.mark_allocated(b, mid);
        alloc.mark_allocated(a, small);

        println!("{:?}", alloc);

        alloc.free(a, small);
        alloc.free(b, mid);
        alloc.mark_allocated(Ptr(24), large);
        let a = alloc.malloc(small).unwrap();
        let b1 = alloc.malloc(small).unwrap();
        let b2 = alloc.malloc(small).unwrap();
        assert_eq!(a, Ptr(0));
        assert_eq!(b1, Ptr(8));
        assert_eq!(b2, Ptr(16));
    }
}
