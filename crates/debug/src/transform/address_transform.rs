use crate::WasmFileInfo;
use gimli::write;
use more_asserts::assert_le;
use std::collections::BTreeMap;
use std::iter::FromIterator;
use wasmtime_environ::entity::{EntityRef, PrimaryMap};
use wasmtime_environ::ir::SourceLoc;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::{FunctionAddressMap, ModuleAddressMap};

pub type GeneratedAddress = usize;
pub type WasmAddress = u64;

/// Contains mapping of the generated address to its original
/// source location.
#[derive(Debug)]
pub struct AddressMap {
    pub generated: GeneratedAddress,
    pub wasm: WasmAddress,
}

/// Information about generated function code: its body start,
/// length, and instructions addresses.
#[derive(Debug)]
pub struct FunctionMap {
    pub offset: GeneratedAddress,
    pub len: GeneratedAddress,
    pub wasm_start: WasmAddress,
    pub wasm_end: WasmAddress,
    pub addresses: Box<[AddressMap]>,
}

/// Mapping of the source location to its generated code range.
#[derive(Debug)]
struct Position {
    wasm_pos: WasmAddress,
    gen_start: GeneratedAddress,
    gen_end: GeneratedAddress,
}

/// Mapping of continuous range of source location to its generated
/// code. The positions are always in ascending order for search.
#[derive(Debug)]
struct Range {
    wasm_start: WasmAddress,
    wasm_end: WasmAddress,
    gen_start: GeneratedAddress,
    gen_end: GeneratedAddress,
    positions: Box<[Position]>,
}

type RangeIndex = usize;

/// Helper function address lookup data. Contains ranges start positions
/// index and ranges data. The multiple ranges can include the same
/// original source position. The index (B-Tree) uses range start
/// position as a key. The index values reference the ranges array.
/// The item are ordered RangeIndex.
#[derive(Debug)]
struct FuncLookup {
    index: Vec<(WasmAddress, Box<[RangeIndex]>)>,
    ranges: Box<[Range]>,
}

/// Mapping of original functions to generated code locations/ranges.
#[derive(Debug)]
struct FuncTransform {
    start: WasmAddress,
    end: WasmAddress,
    index: DefinedFuncIndex,
    lookup: FuncLookup,
}

/// Module functions mapping to generated code.
#[derive(Debug)]
pub struct AddressTransform {
    map: PrimaryMap<DefinedFuncIndex, FunctionMap>,
    func: Vec<(WasmAddress, FuncTransform)>,
}

/// Returns a wasm bytecode offset in the code section from SourceLoc.
pub fn get_wasm_code_offset(loc: SourceLoc, code_section_offset: u64) -> WasmAddress {
    // Code section size <= 4GB, allow wrapped SourceLoc to recover the overflow.
    loc.bits().wrapping_sub(code_section_offset as u32) as WasmAddress
}

fn build_function_lookup(
    ft: &FunctionAddressMap,
    code_section_offset: u64,
) -> (WasmAddress, WasmAddress, FuncLookup) {
    assert_le!(code_section_offset, ft.start_srcloc.bits() as u64);
    let fn_start = get_wasm_code_offset(ft.start_srcloc, code_section_offset);
    let fn_end = get_wasm_code_offset(ft.end_srcloc, code_section_offset);
    assert_le!(fn_start, fn_end);

    // Build ranges of continuous source locations. The new ranges starts when
    // non-descending order is interrupted. Assuming the same origin location can
    // be present in multiple ranges.
    let mut range_wasm_start = fn_start;
    let mut range_gen_start = ft.body_offset;
    let mut last_wasm_pos = range_wasm_start;
    let mut ranges = Vec::new();
    let mut ranges_index = BTreeMap::new();
    let mut current_range = Vec::new();
    let mut last_gen_inst_empty = false;
    for t in &ft.instructions {
        if t.srcloc.is_default() {
            continue;
        }

        let offset = get_wasm_code_offset(t.srcloc, code_section_offset);
        assert_le!(fn_start, offset);
        assert_le!(offset, fn_end);

        let inst_gen_start = t.code_offset;
        let inst_gen_end = t.code_offset + t.code_len;

        if last_wasm_pos > offset {
            // Start new range.
            ranges_index.insert(range_wasm_start, ranges.len());
            ranges.push(Range {
                wasm_start: range_wasm_start,
                wasm_end: last_wasm_pos,
                gen_start: range_gen_start,
                gen_end: inst_gen_start,
                positions: current_range.into_boxed_slice(),
            });
            range_wasm_start = offset;
            range_gen_start = inst_gen_start;
            current_range = Vec::new();
            last_gen_inst_empty = false;
        }
        if last_gen_inst_empty && current_range.last().unwrap().gen_start == inst_gen_start {
            // It is possible that previous inst_gen_start == inst_gen_end, so
            // make an attempt to merge all such positions with current one.
            if inst_gen_start < inst_gen_end {
                let last = current_range.last_mut().unwrap();
                last.gen_end = inst_gen_end;
                last_gen_inst_empty = false;
            }
        } else {
            // Continue existing range: add new wasm->generated code position.
            current_range.push(Position {
                wasm_pos: offset,
                gen_start: inst_gen_start,
                gen_end: inst_gen_end,
            });
            // Track if last position was empty (see if-branch above).
            last_gen_inst_empty = inst_gen_start == inst_gen_end;
        }
        last_wasm_pos = offset;
    }
    let last_gen_addr = ft.body_offset + ft.body_len;
    ranges_index.insert(range_wasm_start, ranges.len());
    ranges.push(Range {
        wasm_start: range_wasm_start,
        wasm_end: fn_end,
        gen_start: range_gen_start,
        gen_end: last_gen_addr,
        positions: current_range.into_boxed_slice(),
    });

    // Making ranges lookup faster by building index: B-tree with every range
    // start position that maps into list of active ranges at this position.
    let ranges = ranges.into_boxed_slice();
    let mut active_ranges = Vec::new();
    let mut index = BTreeMap::new();
    let mut last_wasm_pos = None;
    for (wasm_start, range_index) in ranges_index {
        if Some(wasm_start) == last_wasm_pos {
            active_ranges.push(range_index);
            continue;
        }
        if let Some(position) = last_wasm_pos {
            let mut sorted_ranges = active_ranges.clone();
            sorted_ranges.sort();
            index.insert(position, sorted_ranges.into_boxed_slice());
        }
        active_ranges.retain(|r| ranges[*r].wasm_end.cmp(&wasm_start) != std::cmp::Ordering::Less);
        active_ranges.push(range_index);
        last_wasm_pos = Some(wasm_start);
    }
    active_ranges.sort();
    index.insert(last_wasm_pos.unwrap(), active_ranges.into_boxed_slice());
    let index = Vec::from_iter(index.into_iter());
    (fn_start, fn_end, FuncLookup { index, ranges })
}

fn build_function_addr_map(
    at: &ModuleAddressMap,
    code_section_offset: u64,
) -> PrimaryMap<DefinedFuncIndex, FunctionMap> {
    let mut map = PrimaryMap::new();
    for (_, ft) in at {
        let mut fn_map = Vec::new();
        for t in &ft.instructions {
            if t.srcloc.is_default() {
                continue;
            }
            let offset = get_wasm_code_offset(t.srcloc, code_section_offset);
            fn_map.push(AddressMap {
                generated: t.code_offset,
                wasm: offset,
            });
        }

        if cfg!(debug) {
            // fn_map is sorted by the generated field -- see FunctionAddressMap::instructions.
            for i in 1..fn_map.len() {
                assert_le!(fn_map[i - 1].generated, fn_map[i].generated);
            }
        }

        map.push(FunctionMap {
            offset: ft.body_offset,
            len: ft.body_len,
            wasm_start: get_wasm_code_offset(ft.start_srcloc, code_section_offset),
            wasm_end: get_wasm_code_offset(ft.end_srcloc, code_section_offset),
            addresses: fn_map.into_boxed_slice(),
        });
    }
    map
}

// Utility iterator to find all ranges starts for specific Wasm address.
// The iterator returns generated addresses sorted by RangeIndex.
struct TransformRangeStartIter<'a> {
    addr: WasmAddress,
    indices: &'a [RangeIndex],
    ranges: &'a [Range],
}

impl<'a> TransformRangeStartIter<'a> {
    fn new(func: &'a FuncTransform, addr: WasmAddress) -> Self {
        let found = match func
            .lookup
            .index
            .binary_search_by(|entry| entry.0.cmp(&addr))
        {
            Ok(i) => Some(&func.lookup.index[i].1),
            Err(i) => {
                if i > 0 {
                    Some(&func.lookup.index[i - 1].1)
                } else {
                    None
                }
            }
        };
        if let Some(range_indices) = found {
            TransformRangeStartIter {
                addr,
                indices: range_indices,
                ranges: &func.lookup.ranges,
            }
        } else {
            unreachable!();
        }
    }
}

impl<'a> Iterator for TransformRangeStartIter<'a> {
    type Item = (GeneratedAddress, RangeIndex);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((first, tail)) = self.indices.split_first() {
            let range_index = *first;
            let range = &self.ranges[range_index];
            self.indices = tail;
            let address = match range
                .positions
                .binary_search_by(|a| a.wasm_pos.cmp(&self.addr))
            {
                Ok(i) => range.positions[i].gen_start,
                Err(i) => {
                    if i == 0 {
                        range.gen_start
                    } else {
                        range.positions[i - 1].gen_end
                    }
                }
            };
            Some((address, range_index))
        } else {
            None
        }
    }
}

// Utility iterator to find all ranges ends for specific Wasm address.
// The iterator returns generated addresses sorted by RangeIndex.
struct TransformRangeEndIter<'a> {
    addr: WasmAddress,
    indices: &'a [RangeIndex],
    ranges: &'a [Range],
}

impl<'a> TransformRangeEndIter<'a> {
    fn new(func: &'a FuncTransform, addr: WasmAddress) -> Self {
        let found = match func
            .lookup
            .index
            .binary_search_by(|entry| entry.0.cmp(&addr))
        {
            Ok(i) => Some(&func.lookup.index[i].1),
            Err(i) => {
                if i > 0 {
                    Some(&func.lookup.index[i - 1].1)
                } else {
                    None
                }
            }
        };
        if let Some(range_indices) = found {
            TransformRangeEndIter {
                addr,
                indices: range_indices,
                ranges: &func.lookup.ranges,
            }
        } else {
            unreachable!();
        }
    }
}

impl<'a> Iterator for TransformRangeEndIter<'a> {
    type Item = (GeneratedAddress, RangeIndex);
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((first, tail)) = self.indices.split_first() {
            let range_index = *first;
            let range = &self.ranges[range_index];
            self.indices = tail;
            if range.wasm_start >= self.addr {
                continue;
            }
            let address = match range
                .positions
                .binary_search_by(|a| a.wasm_pos.cmp(&self.addr))
            {
                Ok(i) => range.positions[i].gen_end,
                Err(i) => {
                    if i == range.positions.len() {
                        range.gen_end
                    } else {
                        range.positions[i].gen_start
                    }
                }
            };
            return Some((address, range_index));
        }
        None
    }
}

// Utility iterator to iterate by translated function ranges.
pub struct TransformRangeIter<'a> {
    func: &'a FuncTransform,
    start_it: TransformRangeStartIter<'a>,
    end_it: TransformRangeEndIter<'a>,
    last_start: Option<(GeneratedAddress, RangeIndex)>,
    last_end: Option<(GeneratedAddress, RangeIndex)>,
    last_item: Option<(GeneratedAddress, GeneratedAddress)>,
}

impl<'a> TransformRangeIter<'a> {
    fn new(func: &'a FuncTransform, start: WasmAddress, end: WasmAddress) -> Self {
        let mut start_it = TransformRangeStartIter::new(func, start);
        let last_start = start_it.next();
        let mut end_it = TransformRangeEndIter::new(func, end);
        let last_end = end_it.next();
        TransformRangeIter {
            func,
            start_it,
            end_it,
            last_start,
            last_end,
            last_item: None,
        }
    }
}

impl<'a> Iterator for TransformRangeIter<'a> {
    type Item = (GeneratedAddress, GeneratedAddress);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Merge TransformRangeStartIter and TransformRangeEndIter data using
            // FuncLookup index's field propery to be sorted by RangeIndex.
            let (start, end, range_index): (
                Option<GeneratedAddress>,
                Option<GeneratedAddress>,
                RangeIndex,
            ) = {
                match (self.last_start.as_ref(), self.last_end.as_ref()) {
                    (Some((s, sri)), Some((e, eri))) => {
                        if sri == eri {
                            // Start and end RangeIndex matched.
                            (Some(*s), Some(*e), *sri)
                        } else if sri < eri {
                            (Some(*s), None, *sri)
                        } else {
                            (None, Some(*e), *eri)
                        }
                    }
                    (Some((s, sri)), None) => (Some(*s), None, *sri),
                    (None, Some((e, eri))) => (None, Some(*e), *eri),
                    (None, None) => {
                        // Reached ends for start and end iterators.
                        return None;
                    }
                }
            };
            let range_start = match start {
                Some(range_start) => {
                    // Consume start iterator.
                    self.last_start = self.start_it.next();
                    range_start
                }
                None => {
                    let range = &self.func.lookup.ranges[range_index];
                    range.gen_start
                }
            };
            let range_end = match end {
                Some(range_end) => {
                    // Consume end iterator.
                    self.last_end = self.end_it.next();
                    range_end
                }
                None => {
                    let range = &self.func.lookup.ranges[range_index];
                    range.gen_end
                }
            };

            if cfg!(debug_assertions) {
                match self.last_item.replace((range_start, range_end)) {
                    Some((_, last_end)) => debug_assert!(last_end <= range_start),
                    None => (),
                }
            }

            if range_start < range_end {
                return Some((range_start, range_end));
            }
            // Throw away empty ranges.
            debug_assert!(range_start == range_end);
        }
    }
}

impl AddressTransform {
    pub fn new(at: &ModuleAddressMap, wasm_file: &WasmFileInfo) -> Self {
        let code_section_offset = wasm_file.code_section_offset;

        let mut func = BTreeMap::new();
        for (i, ft) in at {
            let (fn_start, fn_end, lookup) = build_function_lookup(ft, code_section_offset);

            func.insert(
                fn_start,
                FuncTransform {
                    start: fn_start,
                    end: fn_end,
                    index: i,
                    lookup,
                },
            );
        }

        let map = build_function_addr_map(at, code_section_offset);
        let func = Vec::from_iter(func.into_iter());
        AddressTransform { map, func }
    }

    fn find_func(&self, addr: u64) -> Option<&FuncTransform> {
        // TODO check if we need to include end address
        let func = match self.func.binary_search_by(|entry| entry.0.cmp(&addr)) {
            Ok(i) => &self.func[i].1,
            Err(i) => {
                if i > 0 {
                    &self.func[i - 1].1
                } else {
                    return None;
                }
            }
        };
        if addr >= func.start {
            return Some(func);
        }
        None
    }

    pub fn find_func_index(&self, addr: u64) -> Option<DefinedFuncIndex> {
        self.find_func(addr).map(|f| f.index)
    }

    pub fn translate_raw(&self, addr: u64) -> Option<(DefinedFuncIndex, GeneratedAddress)> {
        if addr == 0 {
            // It's normally 0 for debug info without the linked code.
            return None;
        }
        if let Some(func) = self.find_func(addr) {
            if addr == func.end {
                // Clamp last address to the end to extend translation to the end
                // of the function.
                let map = &self.map[func.index];
                return Some((func.index, map.len));
            }
            let first_result = TransformRangeStartIter::new(func, addr).next();
            first_result.map(|(address, _)| (func.index, address))
        } else {
            // Address was not found: function was not compiled?
            None
        }
    }

    pub fn can_translate_address(&self, addr: u64) -> bool {
        self.translate(addr).is_some()
    }

    pub fn translate(&self, addr: u64) -> Option<write::Address> {
        self.translate_raw(addr)
            .map(|(func_index, address)| write::Address::Symbol {
                symbol: func_index.index(),
                addend: address as i64,
            })
    }

    pub fn translate_ranges_raw<'a>(
        &'a self,
        start: u64,
        end: u64,
    ) -> Option<(DefinedFuncIndex, impl Iterator<Item = (usize, usize)> + 'a)> {
        if start == 0 {
            // It's normally 0 for debug info without the linked code.
            return None;
        }
        if let Some(func) = self.find_func(start) {
            let result = TransformRangeIter::new(func, start, end);
            return Some((func.index, result));
        }
        // Address was not found: function was not compiled?
        None
    }

    pub fn translate_ranges<'a>(
        &'a self,
        start: u64,
        end: u64,
    ) -> impl Iterator<Item = (write::Address, u64)> + 'a {
        enum TranslateRangesResult<'a> {
            Empty,
            Raw {
                symbol: usize,
                it: Box<dyn Iterator<Item = (usize, usize)> + 'a>,
            },
        }
        impl<'a> Iterator for TranslateRangesResult<'a> {
            type Item = (write::Address, u64);
            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    TranslateRangesResult::Empty => None,
                    TranslateRangesResult::Raw { symbol, it } => match it.next() {
                        Some((start, end)) => {
                            debug_assert!(start < end);
                            Some((
                                write::Address::Symbol {
                                    symbol: *symbol,
                                    addend: start as i64,
                                },
                                (end - start) as u64,
                            ))
                        }
                        None => None,
                    },
                }
            }
        }

        match self.translate_ranges_raw(start, end) {
            Some((func_index, ranges)) => TranslateRangesResult::Raw {
                symbol: func_index.index(),
                it: Box::new(ranges),
            },
            None => TranslateRangesResult::Empty,
        }
    }

    pub fn map(&self) -> &PrimaryMap<DefinedFuncIndex, FunctionMap> {
        &self.map
    }

    pub fn func_range(&self, index: DefinedFuncIndex) -> (GeneratedAddress, GeneratedAddress) {
        let map = &self.map[index];
        (map.offset, map.offset + map.len)
    }

    pub fn func_source_range(&self, index: DefinedFuncIndex) -> (WasmAddress, WasmAddress) {
        let map = &self.map[index];
        (map.wasm_start, map.wasm_end)
    }
}

#[cfg(test)]
mod tests {
    use super::{build_function_lookup, get_wasm_code_offset, AddressTransform};
    use crate::read_debuginfo::WasmFileInfo;
    use gimli::write::Address;
    use std::iter::FromIterator;
    use wasmtime_environ::entity::PrimaryMap;
    use wasmtime_environ::ir::SourceLoc;
    use wasmtime_environ::{FunctionAddressMap, InstructionAddressMap, ModuleAddressMap};

    #[test]
    fn test_get_wasm_code_offset() {
        let offset = get_wasm_code_offset(SourceLoc::new(3), 1);
        assert_eq!(2, offset);
        let offset = get_wasm_code_offset(SourceLoc::new(16), 0xF000_0000);
        assert_eq!(0x1000_0010, offset);
        let offset = get_wasm_code_offset(SourceLoc::new(1), 0x20_8000_0000);
        assert_eq!(0x8000_0001, offset);
    }

    fn create_simple_func(wasm_offset: u32) -> FunctionAddressMap {
        FunctionAddressMap {
            instructions: vec![
                InstructionAddressMap {
                    srcloc: SourceLoc::new(wasm_offset + 2),
                    code_offset: 5,
                    code_len: 3,
                },
                InstructionAddressMap {
                    srcloc: SourceLoc::new(wasm_offset + 7),
                    code_offset: 15,
                    code_len: 8,
                },
            ],
            start_srcloc: SourceLoc::new(wasm_offset),
            end_srcloc: SourceLoc::new(wasm_offset + 10),
            body_offset: 0,
            body_len: 30,
        }
    }

    fn create_simple_module(func: FunctionAddressMap) -> ModuleAddressMap {
        PrimaryMap::from_iter(vec![func])
    }

    #[test]
    fn test_build_function_lookup_simple() {
        let input = create_simple_func(11);
        let (start, end, lookup) = build_function_lookup(&input, 1);
        assert_eq!(10, start);
        assert_eq!(20, end);

        assert_eq!(1, lookup.index.len());
        let index_entry = lookup.index.into_iter().next().unwrap();
        assert_eq!((10u64, vec![0].into_boxed_slice()), index_entry);
        assert_eq!(1, lookup.ranges.len());
        let range = &lookup.ranges[0];
        assert_eq!(10, range.wasm_start);
        assert_eq!(20, range.wasm_end);
        assert_eq!(0, range.gen_start);
        assert_eq!(30, range.gen_end);
        let positions = &range.positions;
        assert_eq!(2, positions.len());
        assert_eq!(12, positions[0].wasm_pos);
        assert_eq!(5, positions[0].gen_start);
        assert_eq!(8, positions[0].gen_end);
        assert_eq!(17, positions[1].wasm_pos);
        assert_eq!(15, positions[1].gen_start);
        assert_eq!(23, positions[1].gen_end);
    }

    #[test]
    fn test_build_function_lookup_two_ranges() {
        let mut input = create_simple_func(11);
        // append instruction with same srcloc as input.instructions[0]
        input.instructions.push(InstructionAddressMap {
            srcloc: SourceLoc::new(11 + 2),
            code_offset: 23,
            code_len: 3,
        });
        let (start, end, lookup) = build_function_lookup(&input, 1);
        assert_eq!(10, start);
        assert_eq!(20, end);

        assert_eq!(2, lookup.index.len());
        let index_entries = Vec::from_iter(lookup.index.into_iter());
        assert_eq!((10u64, vec![0].into_boxed_slice()), index_entries[0]);
        assert_eq!((12u64, vec![0, 1].into_boxed_slice()), index_entries[1]);
        assert_eq!(2, lookup.ranges.len());

        let range = &lookup.ranges[0];
        assert_eq!(10, range.wasm_start);
        assert_eq!(17, range.wasm_end);
        assert_eq!(0, range.gen_start);
        assert_eq!(23, range.gen_end);
        let positions = &range.positions;
        assert_eq!(2, positions.len());
        assert_eq!(12, positions[0].wasm_pos);
        assert_eq!(5, positions[0].gen_start);
        assert_eq!(8, positions[0].gen_end);
        assert_eq!(17, positions[1].wasm_pos);
        assert_eq!(15, positions[1].gen_start);
        assert_eq!(23, positions[1].gen_end);

        let range = &lookup.ranges[1];
        assert_eq!(12, range.wasm_start);
        assert_eq!(20, range.wasm_end);
        assert_eq!(23, range.gen_start);
        assert_eq!(30, range.gen_end);
        let positions = &range.positions;
        assert_eq!(1, positions.len());
        assert_eq!(12, positions[0].wasm_pos);
        assert_eq!(23, positions[0].gen_start);
        assert_eq!(26, positions[0].gen_end);
    }

    #[test]
    fn test_addr_translate() {
        let input = create_simple_module(create_simple_func(11));
        let at = AddressTransform::new(
            &input,
            &WasmFileInfo {
                path: None,
                code_section_offset: 1,
                imported_func_count: 0,
                funcs: Box::new([]),
            },
        );

        let addr = at.translate(10);
        assert_eq!(
            Some(Address::Symbol {
                symbol: 0,
                addend: 0,
            }),
            addr
        );

        let addr = at.translate(20);
        assert_eq!(
            Some(Address::Symbol {
                symbol: 0,
                addend: 30,
            }),
            addr
        );

        let addr = at.translate(0);
        assert_eq!(None, addr);

        let addr = at.translate(12);
        assert_eq!(
            Some(Address::Symbol {
                symbol: 0,
                addend: 5,
            }),
            addr
        );

        let addr = at.translate(18);
        assert_eq!(
            Some(Address::Symbol {
                symbol: 0,
                addend: 23,
            }),
            addr
        );
    }
}
