//! This file is focused on iterating through the frame stack,
//! and finding all the live object references.

use std::cmp::Ordering;
use std::collections::LinkedList;
use std::fmt::Display;
use std::ops::ControlFlow;
use std::sync::{LazyLock, OnceLock, RwLock};

/// Stack-map for a given function.
///
/// The vector defines a list of tuples containing the offset
/// of the stack map relative to the start of the function, as well
/// as all spilled GC references at that specific address.
///
/// The spilled GC references are defined as a list of offsets,
/// relative to the stack pointer which contain a reference to a living
/// GC reference.
pub type FunctionStackMap = Vec<(usize, usize, Vec<usize>)>;

/// Metadata entry for a single compiled function.
#[derive(Debug)]
pub struct CompiledFunctionMetadata {
    /// Defines the address of the first instruction in the function.
    pub start: *const u8,

    /// Defines the address of the last instruction in the function.
    pub end: *const u8,

    /// Defines a list of all stack maps found within the function,
    /// keyed by offset from [`CompiledFunctionMetadata::start`].
    pub stack_locations: FunctionStackMap,
}

impl CompiledFunctionMetadata {
    /// Gets the [`Ordering`] of the given address, in reference to the interval of
    /// the current metadata entry. This method is used for iterating over a list of
    /// metadata entries using a binary search.
    ///
    /// The truth table for the method is as such[^note]:
    ///
    /// | Input                      | Output ([`Ordering`]) |
    /// |----------------------------|-----------------------|
    /// | `start` > `addr`           | [`Ordering::Greater`] |
    /// | `end` < `addr`             | [`Ordering::Less`]    |
    /// | `start` <= `addr` <= `end` | [`Ordering::Equal`]   |
    ///
    /// [^note]: `start` and `end` denotes the `start` and `end` field in
    /// [`CompiledFunctionMetadata`], respectively.
    #[inline]
    pub fn ordering_of(&self, addr: *const u8) -> Ordering {
        if self.start > addr {
            Ordering::Greater
        } else if addr > self.end {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

unsafe impl Send for CompiledFunctionMetadata {}
unsafe impl Sync for CompiledFunctionMetadata {}

static FUNC_STACK_MAPS: OnceLock<Vec<CompiledFunctionMetadata>> = OnceLock::new();

/// Declares the stack maps for all generated functions in the runtime.
///
/// # Panics
///
/// This function **will** panic if the stack maps are declared more than once.
pub fn declare_stack_maps(mut stack_maps: Vec<CompiledFunctionMetadata>) {
    stack_maps.sort_by_key(|func| func.start.addr());

    FUNC_STACK_MAPS
        .set(stack_maps)
        .expect("function stack maps should only be assigned once");
}

/// Represents a single stack map, corresponding to a specific
/// safepoint location within a compiled Lume function.
#[derive(Debug)]
pub(crate) struct FrameStackMap {
    pub map: &'static CompiledFunctionMetadata,
    pub frame_pointer: *const u8,
    pub program_counter: *const u8,
}

impl FrameStackMap {
    /// Gets the offset of the stack frame from the first
    /// instruction in the associated function.
    #[inline]
    pub(crate) fn offset(&self) -> usize {
        self.program_counter.addr() - self.map.start.addr()
    }

    /// Gets the stack pointer which is associated with the frame.
    #[inline]
    pub(crate) fn stack_pointer(&self) -> *const u8 {
        unsafe {
            crate::arch::parent_frame_pointer(self.frame_pointer)
                .byte_add(crate::arch::PARENT_SP_FROM_FP_OFFSET)
        }
    }

    /// Gets all the stack location offsets of the current frame stack map.
    ///
    /// The returned slice will be a list of offsets relative to the stack pointer
    /// of the frame, which will contain a pointer to a GC reference.
    ///
    /// For more information, see [`stack_locations`] which will get the absolute
    /// addresses of the GC references.
    #[inline]
    pub(crate) fn stack_offsets(&self) -> &[usize] {
        let offset = self.offset();

        self.map
            .stack_locations
            .iter()
            .find_map(|loc| {
                if offset >= loc.0 && loc.0 + loc.1 >= offset {
                    Some(loc.2.as_slice())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| &[])
    }

    /// Attempts to find all GC references found inside of the stack map for the current
    /// program counter. The returned iterator will iterate over a list of pointers,
    /// which point to an item inside the current stack frame.
    ///
    /// To get the address of the underlying allocation, simply read the pointer. This
    /// is to facilitate the GC moving the underlying allocation to a different address,
    /// whereafter it can write the new address to the pointer in the stack frame.
    #[inline]
    pub(crate) fn stack_locations(&self) -> impl Iterator<Item = *const *const u8> {
        self.stack_offsets()
            .iter()
            .map(|offset| unsafe { self.stack_pointer().byte_add(*offset) } as *const *const u8)
    }

    /// Attempts to find all GC references found inside of the stack map for the current
    /// program counter.
    ///
    /// The returned iterator will iterate over a list of tuples. The first element in the
    /// tuple is an entry in the current stack frame containing the GC reference and the
    /// second element is a pointer to the GC reference itself.
    #[inline]
    pub(crate) fn stack_value_locations(
        &self,
    ) -> impl Iterator<Item = (*const *const u8, *const u8)> {
        self.stack_locations().map(|ptr| {
            let gc_ref = unsafe { ptr.read() };

            (ptr, gc_ref)
        })
    }
}

impl Display for FrameStackMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Frame: PC={:p}, FP={:p}, SP={:p}",
            self.program_counter,
            self.frame_pointer,
            self.stack_pointer()
        ))
    }
}

/// Represents an entry in the managed call-stack.
#[derive(Clone, Copy)]
pub(crate) struct FrameEntry {
    pub frame_pointer: *const u8,
    pub program_counter: *const u8,
}

impl FrameEntry {
    /// Attempts to find the stack map for the function, which matches the
    /// current frame entry. If no function is found for the given entry or if
    /// no stack map is attached to the found function, returns [`None`].
    ///
    /// # Panics
    ///
    /// This function will panic if the stack maps have not yet been declared. To declare
    /// them, use [`declare_stack_maps`].
    fn find_stack_map(self) -> Option<FrameStackMap> {
        let stack_maps = FUNC_STACK_MAPS
            .get()
            .expect("expected function stack map to be set");

        if let Ok(idx) =
            stack_maps.binary_search_by(|probe| probe.ordering_of(self.program_counter))
        {
            let stack_map = stack_maps
                .get(idx)
                .expect("expected index to exist after search");

            return Some(FrameStackMap {
                map: stack_map,
                frame_pointer: self.frame_pointer,
                program_counter: self.program_counter,
            });
        }

        None
    }
}

impl Display for FrameEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Frame entry: PC={:p}, FP={:p}",
            self.program_counter, self.frame_pointer
        ))
    }
}

unsafe impl Send for FrameEntry {}
unsafe impl Sync for FrameEntry {}

/// The globally-available managed call-stack.
///
/// The managed call-stack is a linked-list of all entry
/// frames (calls from host-to-JIT) and exit frames (calls from JIT-to-host).
///
/// This stack allows for performant stack walking without having to inspect
/// hardware registers or rely on compliant usage of frame pointers.
static FRAME_STACK: LazyLock<RwLock<LinkedList<FrameEntry>>> =
    LazyLock::new(|| RwLock::new(LinkedList::new()));

/// Push a new frame entry onto the managed call stack.
///
/// Frame entries get pushed whenever a call from host-to-JIT or JIT-to-host
/// is made, so we can walk the frame stack.
pub(crate) fn push_frame_entry(frame_pointer: *const u8, program_counter: *const u8) {
    FRAME_STACK.try_write().unwrap().push_front(FrameEntry {
        frame_pointer,
        program_counter,
    });
}

/// Pop the top-level frame entry off the managed call stack.
pub(crate) fn pop_frame_entry() {
    FRAME_STACK
        .try_write()
        .unwrap()
        .pop_front()
        .expect("attempted to exit frame without corresponding entry");
}

/// Walk the current frame stack, calling `f` with a matching
/// pair of entry- and exit-frames, as we walk.
pub(crate) fn visit_chunked_frames<T>(
    mut f: impl FnMut(FrameEntry, FrameEntry) -> ControlFlow<T>,
) -> Option<T> {
    let frames = FRAME_STACK.try_read().unwrap();
    let mut frame_iter = frames.iter();

    loop {
        let exit = frame_iter.next()?;
        let entry = frame_iter.next()?;

        if let ControlFlow::Break(val) = f(*entry, *exit) {
            return Some(val);
        }
    }
}

/// Walk the current frame stack, calling `f` for each frame we walk.
pub(crate) fn visit_frames<T>(mut f: impl FnMut(FrameEntry) -> ControlFlow<T>) -> Option<T> {
    visit_chunked_frames(|entry, exit| {
        let mut fp = exit.frame_pointer;

        while fp != entry.frame_pointer {
            // The exit frame pointer should always be a sub-frame of
            // the entry frame.
            debug_assert!(fp <= entry.frame_pointer);

            let pc = unsafe { crate::arch::return_addr_of_frame(fp) };

            let entry = FrameEntry {
                frame_pointer: fp,
                program_counter: pc,
            };

            if let ControlFlow::Break(value) = f(entry) {
                return ControlFlow::Break(value);
            }

            fp = unsafe { crate::arch::parent_frame_pointer(fp) };
        }

        ControlFlow::Continue(())
    })
}

/// Attempts to find a frame stack map which corresponds to the current frame pointer.
///
/// If no frame stack map can be found for the current frame pointer, the function
/// iterates through all parent frames, until a frame stack map is found.
///
/// If no frame stack maps are found in any parent frames, the functions returns [`None`].
#[inline]
pub(crate) fn find_current_stack_map() -> Option<FrameStackMap> {
    visit_frames(|frame| {
        if let Some(stack_map) = frame.find_stack_map() {
            return ControlFlow::Break(stack_map);
        }

        ControlFlow::Continue(())
    })
}
