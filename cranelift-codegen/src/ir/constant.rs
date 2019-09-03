//! Constants
//!
//! The constant pool defined here allows Cranelift to avoid emitting the same constant multiple
//! times. As constants are inserted in the pool, a handle is returned; the handle is a Cranelift
//! Entity. Inserting the same data multiple times will always return the same handle.
//!
//! Future work could include:
//! - ensuring alignment of constants within the pool,
//! - bucketing constants by size.

use crate::ir::Constant;
use cranelift_entity::EntityRef;
use std::collections::{BTreeMap, HashMap};
use std::vec::Vec;

/// This type describes the actual constant data.
pub type ConstantData = Vec<u8>;

/// This type describes an offset in bytes within a constant pool.
pub type ConstantOffset = u32;

/// Inner type for storing data and offset together in the constant pool. The offset is optional
/// because it must be set relative to the function code size (i.e. constants are emitted after the
/// function body); because the function is not yet compiled when constants are inserted,
/// [`set_offset`](crate::ir::ConstantPool::set_offset) must be called once a constant's offset
/// from the beginning of the function is known (see
/// [`relaxation.rs`](crate::binemit::relaxation)).
#[derive(Clone)]
pub struct ConstantPoolEntry {
    data: ConstantData,
    offset: Option<ConstantOffset>,
}

impl ConstantPoolEntry {
    fn new(data: ConstantData) -> Self {
        Self { data, offset: None }
    }

    /// Return the size of the constant at this entry.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Assign a new offset to the constant at this entry.
    pub fn set_offset(&mut self, offset: ConstantOffset) {
        self.offset = Some(offset)
    }
}

/// Maintains the mapping between a constant handle (i.e.  [`Constant`](crate::ir::Constant)) and
/// its constant data (i.e.  [`ConstantData`](crate::ir::ConstantData)).
#[derive(Clone)]
pub struct ConstantPool {
    /// This mapping maintains the insertion order as long as Constants are created with
    /// sequentially increasing integers.
    handles_to_values: BTreeMap<Constant, ConstantPoolEntry>,

    /// This mapping is unordered (no need for lexicographic ordering) but allows us to map
    /// constant data back to handles.
    values_to_handles: HashMap<ConstantData, Constant>,
}

impl ConstantPool {
    /// Create a new constant pool instance.
    pub fn new() -> Self {
        Self {
            handles_to_values: BTreeMap::new(),
            values_to_handles: HashMap::new(),
        }
    }

    /// Empty the constant pool of all data.
    pub fn clear(&mut self) {
        self.handles_to_values.clear();
        self.values_to_handles.clear();
    }

    /// Insert constant data into the pool, returning a handle for later referencing; when constant
    /// data is inserted that is a duplicate of previous constant data, the existing handle will be
    /// returned.
    pub fn insert(&mut self, constant_value: ConstantData) -> Constant {
        if self.values_to_handles.contains_key(&constant_value) {
            self.values_to_handles.get(&constant_value).unwrap().clone()
        } else {
            let constant_handle = Constant::new(self.len());
            self.values_to_handles
                .insert(constant_value.clone(), constant_handle.clone());
            self.handles_to_values.insert(
                constant_handle.clone(),
                ConstantPoolEntry::new(constant_value),
            );
            constant_handle
        }
    }

    /// Retrieve the constant data given a handle.
    pub fn get(&self, constant_handle: Constant) -> &ConstantData {
        assert!(self.handles_to_values.contains_key(&constant_handle));
        &self.handles_to_values.get(&constant_handle).unwrap().data
    }

    /// Assign an offset to a given constant, where the offset is the number of bytes from the
    /// beginning of the function to the beginning of the constant data inside the pool.
    pub fn set_offset(&mut self, constant_handle: Constant, constant_offset: ConstantOffset) {
        assert!(
            self.handles_to_values.contains_key(&constant_handle),
            "A constant handle must have already been inserted into the pool; perhaps a \
             constant pool was created outside of the pool?"
        );
        self.handles_to_values
            .entry(constant_handle)
            .and_modify(|e| e.offset = Some(constant_offset));
    }

    /// Retrieve the offset of a given constant, where the offset is the number of bytes from the
    /// beginning of the function to the beginning of the constant data inside the pool.
    pub fn get_offset(&self, constant_handle: Constant) -> ConstantOffset {
        self.handles_to_values
            .get(&constant_handle)
            .expect(
                "A constant handle must have a corresponding constant value; was a constant \
                 handle created outside of the pool?",
            )
            .offset
            .expect(
                "A constant offset has not yet been set; verify that `set_offset` has been \
                 called before this point",
            )
    }

    /// Iterate over the constants in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&Constant, &ConstantData)> {
        self.handles_to_values.iter().map(|(h, e)| (h, &e.data))
    }

    /// Iterate over mutable entries in the constant pool in insertion order.
    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut ConstantPoolEntry> {
        self.handles_to_values.values_mut()
    }

    /// Return the number of constants in the pool.
    pub fn len(&self) -> usize {
        self.handles_to_values.len()
    }

    /// Return the combined size of all of the constant values in the pool.
    pub fn byte_size(&self) -> usize {
        self.values_to_handles.keys().map(|c| c.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let sut = ConstantPool::new();
        assert_eq!(sut.len(), 0);
    }

    #[test]
    fn insert() {
        let mut sut = ConstantPool::new();
        sut.insert(vec![1, 2, 3]);
        sut.insert(vec![4, 5, 6]);
        assert_eq!(sut.len(), 2);
    }

    #[test]
    fn insert_duplicate() {
        let mut sut = ConstantPool::new();
        let a = sut.insert(vec![1, 2, 3]);
        sut.insert(vec![4, 5, 6]);
        let b = sut.insert(vec![1, 2, 3]);
        assert_eq!(a, b);
    }

    #[test]
    fn clear() {
        let mut sut = ConstantPool::new();
        sut.insert(vec![1, 2, 3]);
        assert_eq!(sut.len(), 1);

        sut.clear();
        assert_eq!(sut.len(), 0);
    }

    #[test]
    fn iteration_order() {
        let mut sut = ConstantPool::new();
        sut.insert(vec![1, 2, 3]);
        sut.insert(vec![4, 5, 6]);
        sut.insert(vec![1, 2, 3]);
        let data = sut.iter().map(|(_, v)| v).collect::<Vec<&ConstantData>>();
        assert_eq!(data, vec![&vec![1, 2, 3], &vec![4, 5, 6]]);
    }

    #[test]
    fn get() {
        let mut sut = ConstantPool::new();
        let data = vec![1, 2, 3];
        let handle = sut.insert(data.clone());
        assert_eq!(sut.get(handle), &data);
    }

    #[test]
    #[should_panic]
    fn get_nonexistent_constant() {
        let sut = ConstantPool::new();
        let a = Constant::with_number(42).unwrap();
        sut.get(a); // panics, only use constants returned by ConstantPool
    }

    #[test]
    fn get_offset() {
        let mut sut = ConstantPool::new();
        let a = sut.insert(vec![1]);
        sut.set_offset(a, 42);
        assert_eq!(sut.get_offset(a), 42)
    }

    #[test]
    #[should_panic]
    fn get_nonexistent_offset() {
        let mut sut = ConstantPool::new();
        let a = sut.insert(vec![1]);
        sut.get_offset(a); // panics, set_offset should have been called
    }
}
