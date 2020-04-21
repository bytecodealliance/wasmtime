use super::address_transform::AddressTransform;
use anyhow::{Context, Error, Result};
use gimli::{self, write, Expression, Operation, Reader, ReaderOffset, X86_64};
use more_asserts::{assert_le, assert_lt};
use std::collections::{HashMap, HashSet};
use wasmtime_environ::entity::EntityRef;
use wasmtime_environ::ir::{StackSlots, ValueLabel, ValueLabelsRanges, ValueLoc};
use wasmtime_environ::isa::TargetIsa;
use wasmtime_environ::wasm::{get_vmctx_value_label, DefinedFuncIndex};
use wasmtime_environ::ModuleMemoryOffset;

#[derive(Debug)]
pub struct FunctionFrameInfo<'a> {
    pub value_ranges: &'a ValueLabelsRanges,
    pub memory_offset: ModuleMemoryOffset,
    pub stack_slots: &'a StackSlots,
}

impl<'a> FunctionFrameInfo<'a> {
    fn vmctx_memory_offset(&self) -> Option<i64> {
        match self.memory_offset {
            ModuleMemoryOffset::Defined(x) => Some(x as i64),
            ModuleMemoryOffset::Imported(_) => {
                // TODO implement memory offset for imported memory
                None
            }
            ModuleMemoryOffset::None => None,
        }
    }
}

struct ExpressionWriter(write::EndianVec<gimli::RunTimeEndian>);

impl ExpressionWriter {
    pub fn new() -> Self {
        let endian = gimli::RunTimeEndian::Little;
        let writer = write::EndianVec::new(endian);
        ExpressionWriter(writer)
    }

    pub fn write_op(&mut self, op: gimli::DwOp) -> write::Result<()> {
        self.write_u8(op.0 as u8)
    }

    pub fn write_op_reg(&mut self, op: gimli::DwOp, reg: u16) -> write::Result<()> {
        assert!(reg < 32);
        self.write_u8(op.0 as u8 + reg as u8)
    }

    pub fn write_u8(&mut self, b: u8) -> write::Result<()> {
        write::Writer::write_u8(&mut self.0, b)
    }

    pub fn write_uleb128(&mut self, i: u64) -> write::Result<()> {
        write::Writer::write_uleb128(&mut self.0, i)
    }

    pub fn write_sleb128(&mut self, i: i64) -> write::Result<()> {
        write::Writer::write_sleb128(&mut self.0, i)
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0.into_vec()
    }
}

#[derive(Debug, Clone)]
enum CompiledExpressionPart {
    // Untranslated DWARF expression.
    Code(Vec<u8>),
    // The wasm-local DWARF operator. The label points to `ValueLabel`.
    // The trailing field denotes that the operator was last in sequence,
    // and it is the DWARF location (not a pointer).
    Local { label: ValueLabel, trailing: bool },
    // Deference is needed.
    Deref,
}

#[derive(Debug, Clone)]
pub struct CompiledExpression {
    parts: Vec<CompiledExpressionPart>,
    need_deref: bool,
}

impl CompiledExpression {
    pub fn vmctx() -> CompiledExpression {
        CompiledExpression::from_label(get_vmctx_value_label())
    }

    pub fn from_label(label: ValueLabel) -> CompiledExpression {
        CompiledExpression {
            parts: vec![CompiledExpressionPart::Local {
                label,
                trailing: true,
            }],
            need_deref: false,
        }
    }
}

const X86_64_STACK_OFFSET: i64 = 16;

fn translate_loc(
    loc: ValueLoc,
    frame_info: Option<&FunctionFrameInfo>,
    isa: &dyn TargetIsa,
    add_stack_value: bool,
) -> Result<Option<Vec<u8>>> {
    Ok(match loc {
        ValueLoc::Reg(reg) if add_stack_value => {
            let machine_reg = isa.map_dwarf_register(reg)?;
            let mut writer = ExpressionWriter::new();
            if machine_reg < 32 {
                writer.write_op_reg(gimli::constants::DW_OP_reg0, machine_reg)?;
            } else {
                writer.write_op(gimli::constants::DW_OP_regx)?;
                writer.write_uleb128(machine_reg.into())?;
            }
            Some(writer.into_vec())
        }
        ValueLoc::Reg(reg) => {
            assert!(!add_stack_value);
            let machine_reg = isa.map_dwarf_register(reg)?;
            let mut writer = ExpressionWriter::new();
            if machine_reg < 32 {
                writer.write_op_reg(gimli::constants::DW_OP_breg0, machine_reg)?;
            } else {
                writer.write_op(gimli::constants::DW_OP_bregx)?;
                writer.write_uleb128(machine_reg.into())?;
            }
            writer.write_sleb128(0)?;
            Some(writer.into_vec())
        }
        ValueLoc::Stack(ss) => {
            if let Some(frame_info) = frame_info {
                if let Some(ss_offset) = frame_info.stack_slots[ss].offset {
                    let mut writer = ExpressionWriter::new();
                    writer.write_op_reg(gimli::constants::DW_OP_breg0, X86_64::RBP.0)?;
                    writer.write_sleb128(ss_offset as i64 + X86_64_STACK_OFFSET)?;
                    if !add_stack_value {
                        writer.write_op(gimli::constants::DW_OP_deref)?;
                    }
                    return Ok(Some(writer.into_vec()));
                }
            }
            None
        }
        _ => None,
    })
}

fn append_memory_deref(
    buf: &mut Vec<u8>,
    frame_info: &FunctionFrameInfo,
    vmctx_loc: ValueLoc,
    isa: &dyn TargetIsa,
) -> Result<bool> {
    let mut writer = ExpressionWriter::new();
    // FIXME for imported memory
    match vmctx_loc {
        ValueLoc::Reg(vmctx_reg) => {
            let reg = isa.map_dwarf_register(vmctx_reg)? as u8;
            writer.write_u8(gimli::constants::DW_OP_breg0.0 + reg)?;
            let memory_offset = match frame_info.vmctx_memory_offset() {
                Some(offset) => offset,
                None => {
                    return Ok(false);
                }
            };
            writer.write_sleb128(memory_offset)?;
        }
        ValueLoc::Stack(ss) => {
            if let Some(ss_offset) = frame_info.stack_slots[ss].offset {
                writer.write_op_reg(gimli::constants::DW_OP_breg0, X86_64::RBP.0)?;
                writer.write_sleb128(ss_offset as i64 + X86_64_STACK_OFFSET)?;
                writer.write_op(gimli::constants::DW_OP_deref)?;
                writer.write_op(gimli::constants::DW_OP_consts)?;
                let memory_offset = match frame_info.vmctx_memory_offset() {
                    Some(offset) => offset,
                    None => {
                        return Ok(false);
                    }
                };
                writer.write_sleb128(memory_offset)?;
                writer.write_op(gimli::constants::DW_OP_plus)?;
            } else {
                return Ok(false);
            }
        }
        _ => {
            return Ok(false);
        }
    }
    writer.write_op(gimli::constants::DW_OP_deref)?;
    writer.write_op(gimli::constants::DW_OP_swap)?;
    writer.write_op(gimli::constants::DW_OP_constu)?;
    writer.write_uleb128(0xffff_ffff)?;
    writer.write_op(gimli::constants::DW_OP_and)?;
    writer.write_op(gimli::constants::DW_OP_plus)?;
    buf.extend(writer.into_vec());
    Ok(true)
}

impl CompiledExpression {
    pub fn is_simple(&self) -> bool {
        if let [CompiledExpressionPart::Code(_)] = self.parts.as_slice() {
            true
        } else {
            self.parts.is_empty()
        }
    }

    pub fn build(&self) -> Option<write::Expression> {
        if let [CompiledExpressionPart::Code(code)] = self.parts.as_slice() {
            return Some(write::Expression(code.to_vec()));
        }
        // locals found, not supported
        None
    }

    pub fn build_with_locals(
        &self,
        scope: &[(u64, u64)], // wasm ranges
        addr_tr: &AddressTransform,
        frame_info: Option<&FunctionFrameInfo>,
        isa: &dyn TargetIsa,
    ) -> Result<Vec<(write::Address, u64, write::Expression)>> {
        if scope.is_empty() {
            return Ok(vec![]);
        }

        if let [CompiledExpressionPart::Code(code)] = self.parts.as_slice() {
            let mut result_scope = Vec::new();
            for s in scope {
                for (addr, len) in addr_tr.translate_ranges(s.0, s.1) {
                    result_scope.push((addr, len, write::Expression(code.to_vec())));
                }
            }
            return Ok(result_scope);
        }

        let vmctx_label = get_vmctx_value_label();

        // Some locals are present, preparing and divided ranges based on the scope
        // and frame_info data.
        let mut ranges_builder = ValueLabelRangesBuilder::new(scope, addr_tr, frame_info);
        for p in &self.parts {
            match p {
                CompiledExpressionPart::Code(_) => (),
                CompiledExpressionPart::Local { label, .. } => ranges_builder.process_label(*label),
                CompiledExpressionPart::Deref => ranges_builder.process_label(vmctx_label),
            }
        }
        if self.need_deref {
            ranges_builder.process_label(vmctx_label);
        }
        ranges_builder.remove_incomplete_ranges();
        let ranges = ranges_builder.ranges;

        let mut result = Vec::new();
        'range: for CachedValueLabelRange {
            func_index,
            start,
            end,
            label_location,
        } in ranges
        {
            // build expression
            let mut code_buf = Vec::new();
            for part in &self.parts {
                match part {
                    CompiledExpressionPart::Code(c) => code_buf.extend_from_slice(c.as_slice()),
                    CompiledExpressionPart::Local { label, trailing } => {
                        let loc = *label_location.get(&label).context("label_location")?;
                        if let Some(expr) = translate_loc(loc, frame_info, isa, *trailing)? {
                            code_buf.extend_from_slice(&expr)
                        } else {
                            continue 'range;
                        }
                    }
                    CompiledExpressionPart::Deref => {
                        if let (Some(vmctx_loc), Some(frame_info)) =
                            (label_location.get(&vmctx_label), frame_info)
                        {
                            if !append_memory_deref(&mut code_buf, frame_info, *vmctx_loc, isa)? {
                                continue 'range;
                            }
                        } else {
                            continue 'range;
                        };
                    }
                }
            }
            if self.need_deref {
                if let (Some(vmctx_loc), Some(frame_info)) =
                    (label_location.get(&vmctx_label), frame_info)
                {
                    if !append_memory_deref(&mut code_buf, frame_info, *vmctx_loc, isa)? {
                        continue 'range;
                    }
                } else {
                    continue 'range;
                };
            }
            result.push((
                write::Address::Symbol {
                    symbol: func_index.index(),
                    addend: start as i64,
                },
                (end - start) as u64,
                write::Expression(code_buf),
            ));
        }

        Ok(result)
    }
}

fn is_old_expression_format(buf: &[u8]) -> bool {
    // Heuristic to detect old variable expression format without DW_OP_fbreg:
    // DW_OP_plus_uconst op must be present, but not DW_OP_fbreg.
    if buf.contains(&(gimli::constants::DW_OP_fbreg.0 as u8)) {
        // Stop check if DW_OP_fbreg exist.
        return false;
    }
    buf.contains(&(gimli::constants::DW_OP_plus_uconst.0 as u8))
}

pub fn compile_expression<R>(
    expr: &Expression<R>,
    encoding: gimli::Encoding,
    frame_base: Option<&CompiledExpression>,
) -> Result<Option<CompiledExpression>, Error>
where
    R: Reader,
{
    let mut pc = expr.0.clone();
    let buf = expr.0.to_slice()?;
    let mut parts = Vec::new();
    let mut need_deref = false;
    if is_old_expression_format(&buf) && frame_base.is_some() {
        // Still supporting old DWARF variable expressions without fbreg.
        parts.extend_from_slice(&frame_base.unwrap().parts);
        if let Some(CompiledExpressionPart::Local { trailing, .. }) = parts.last_mut() {
            *trailing = false;
        }
        need_deref = frame_base.unwrap().need_deref;
    }
    let base_len = parts.len();
    let mut code_chunk = Vec::new();
    macro_rules! flush_code_chunk {
        () => {
            if !code_chunk.is_empty() {
                parts.push(CompiledExpressionPart::Code(code_chunk));
                code_chunk = Vec::new();
            }
        };
    };
    while !pc.is_empty() {
        let next = buf[pc.offset_from(&expr.0).into_u64() as usize];
        need_deref = true;
        if next == 0xED {
            // WebAssembly DWARF extension
            pc.read_u8()?;
            let ty = pc.read_uleb128()?;
            // Supporting only wasm locals.
            if ty != 0 {
                // TODO support wasm globals?
                return Ok(None);
            }
            let index = pc.read_sleb128()?;
            flush_code_chunk!();
            let label = ValueLabel::from_u32(index as u32);
            parts.push(CompiledExpressionPart::Local {
                label,
                trailing: false,
            });
        } else {
            let pos = pc.offset_from(&expr.0).into_u64() as usize;
            let op = Operation::parse(&mut pc, &expr.0, encoding)?;
            match op {
                Operation::FrameOffset { offset } => {
                    // Expand DW_OP_fpreg into frame location and DW_OP_plus_uconst.
                    if frame_base.is_some() {
                        // Add frame base expressions.
                        flush_code_chunk!();
                        parts.extend_from_slice(&frame_base.unwrap().parts);
                    }
                    if let Some(CompiledExpressionPart::Local { trailing, .. }) = parts.last_mut() {
                        // Reset local trailing flag.
                        *trailing = false;
                    }
                    // Append DW_OP_plus_uconst part.
                    let mut writer = ExpressionWriter::new();
                    writer.write_op(gimli::constants::DW_OP_plus_uconst)?;
                    writer.write_uleb128(offset as u64)?;
                    code_chunk.extend(writer.into_vec());
                    continue;
                }
                Operation::Literal { .. }
                | Operation::PlusConstant { .. }
                | Operation::Piece { .. } => (),
                Operation::StackValue => {
                    need_deref = false;

                    // Find extra stack_value, that follow wasm-local operators,
                    // and mark such locals with special flag.
                    if let (Some(CompiledExpressionPart::Local { trailing, .. }), true) =
                        (parts.last_mut(), code_chunk.is_empty())
                    {
                        *trailing = true;
                        continue;
                    }
                }
                Operation::Deref { .. } => {
                    flush_code_chunk!();
                    parts.push(CompiledExpressionPart::Deref);
                }
                _ => {
                    return Ok(None);
                }
            }
            let chunk = &buf[pos..pc.offset_from(&expr.0).into_u64() as usize];
            code_chunk.extend_from_slice(chunk);
        }
    }

    if !code_chunk.is_empty() {
        parts.push(CompiledExpressionPart::Code(code_chunk));
    }

    if base_len > 0 && base_len + 1 < parts.len() {
        // see if we can glue two code chunks
        if let [CompiledExpressionPart::Code(cc1), CompiledExpressionPart::Code(cc2)] =
            &parts[base_len..=base_len]
        {
            let mut combined = cc1.clone();
            combined.extend_from_slice(cc2);
            parts[base_len] = CompiledExpressionPart::Code(combined);
            parts.remove(base_len + 1);
        }
    }

    Ok(Some(CompiledExpression { parts, need_deref }))
}

#[derive(Debug, Clone)]
struct CachedValueLabelRange {
    func_index: DefinedFuncIndex,
    start: usize,
    end: usize,
    label_location: HashMap<ValueLabel, ValueLoc>,
}

struct ValueLabelRangesBuilder<'a, 'b> {
    ranges: Vec<CachedValueLabelRange>,
    addr_tr: &'a AddressTransform,
    frame_info: Option<&'a FunctionFrameInfo<'b>>,
    processed_labels: HashSet<ValueLabel>,
}

impl<'a, 'b> ValueLabelRangesBuilder<'a, 'b> {
    fn new(
        scope: &[(u64, u64)], // wasm ranges
        addr_tr: &'a AddressTransform,
        frame_info: Option<&'a FunctionFrameInfo<'b>>,
    ) -> Self {
        let mut ranges = Vec::new();
        for s in scope {
            if let Some((func_index, tr)) = addr_tr.translate_ranges_raw(s.0, s.1) {
                for (start, end) in tr {
                    ranges.push(CachedValueLabelRange {
                        func_index,
                        start,
                        end,
                        label_location: HashMap::new(),
                    })
                }
            }
        }
        ranges.sort_unstable_by(|a, b| a.start.cmp(&b.start));
        ValueLabelRangesBuilder {
            ranges,
            addr_tr,
            frame_info,
            processed_labels: HashSet::new(),
        }
    }

    fn process_label(&mut self, label: ValueLabel) {
        if self.processed_labels.contains(&label) {
            return;
        }
        self.processed_labels.insert(label);

        let value_ranges = if let Some(frame_info) = self.frame_info {
            &frame_info.value_ranges
        } else {
            return;
        };

        let ranges = &mut self.ranges;
        if let Some(local_ranges) = value_ranges.get(&label) {
            for local_range in local_ranges {
                let wasm_start = local_range.start;
                let wasm_end = local_range.end;
                let loc = local_range.loc;
                // Find all native ranges for the value label ranges.
                for (addr, len) in self
                    .addr_tr
                    .translate_ranges(wasm_start as u64, wasm_end as u64)
                {
                    let (range_start, range_end) = self.addr_tr.convert_to_code_range(addr, len);
                    if range_start == range_end {
                        continue;
                    }
                    assert_lt!(range_start, range_end);
                    // Find acceptable scope of ranges to intersect with.
                    let i = match ranges.binary_search_by(|s| s.start.cmp(&range_start)) {
                        Ok(i) => i,
                        Err(i) => {
                            if i > 0 && range_start < ranges[i - 1].end {
                                i - 1
                            } else {
                                i
                            }
                        }
                    };
                    let j = match ranges.binary_search_by(|s| s.start.cmp(&range_end)) {
                        Ok(i) | Err(i) => i,
                    };
                    // Starting for the end, intersect (range_start..range_end) with
                    // self.ranges array.
                    for i in (i..j).rev() {
                        if range_end <= ranges[i].start || ranges[i].end <= range_start {
                            continue;
                        }
                        if range_end < ranges[i].end {
                            // Cutting some of the range from the end.
                            let mut tail = ranges[i].clone();
                            ranges[i].end = range_end;
                            tail.start = range_end;
                            ranges.insert(i + 1, tail);
                        }
                        assert_le!(ranges[i].end, range_end);
                        if range_start <= ranges[i].start {
                            ranges[i].label_location.insert(label, loc);
                            continue;
                        }
                        // Cutting some of the range from the start.
                        let mut tail = ranges[i].clone();
                        ranges[i].end = range_start;
                        tail.start = range_start;
                        tail.label_location.insert(label, loc);
                        ranges.insert(i + 1, tail);
                    }
                }
            }
        }
    }

    fn remove_incomplete_ranges(&mut self) {
        // Ranges with not-enough labels are discarded.
        let processed_labels_len = self.processed_labels.len();
        self.ranges
            .retain(|r| r.label_location.len() == processed_labels_len);
    }
}

#[cfg(test)]
mod tests {
    use super::compile_expression;
    use gimli::{self, Encoding, EndianSlice, Expression, RunTimeEndian};

    macro_rules! expression {
        ($($i:literal),*) => {
            Expression(EndianSlice::new(
                &[$($i),*],
                RunTimeEndian::Little,
            ))
        }
    }

    static DWARF_ENCODING: Encoding = Encoding {
        address_size: 4,
        format: gimli::Format::Dwarf32,
        version: 4,
    };

    #[test]
    fn test_debug_parse_expressions() {
        // DW_OP_WASM_location 0x0 +20, DW_OP_stack_value
        let e = expression!(0xed, 0x00, 0x14, 0x9f);
        let ce = compile_expression(&e, DWARF_ENCODING, None)
            .expect("non-error")
            .expect("expression");
        assert_eq!(format!("{:?}", ce), "CompiledExpression { parts: [Local { label: val20, trailing: true }], need_deref: false }");

        //  DW_OP_WASM_location 0x0 +1, DW_OP_plus_uconst 0x10, DW_OP_stack_value
        let e = expression!(0xed, 0x00, 0x01, 0x23, 0x10, 0x9f);
        let ce = compile_expression(&e, DWARF_ENCODING, None)
            .expect("non-error")
            .expect("expression");
        assert_eq!(format!("{:?}", ce), "CompiledExpression { parts: [Local { label: val1, trailing: false }, Code([35, 16, 159])], need_deref: false }");

        // Frame base: DW_OP_WASM_location 0x0 +3, DW_OP_stack_value
        let e = expression!(0xed, 0x00, 0x03, 0x9f);
        let fe = compile_expression(&e, DWARF_ENCODING, None).expect("non-error");
        // DW_OP_fpreg 0x12
        let e = expression!(0x91, 0x12);
        let ce = compile_expression(&e, DWARF_ENCODING, fe.as_ref())
            .expect("non-error")
            .expect("expression");
        assert_eq!(format!("{:?}", ce), "CompiledExpression { parts: [Local { label: val3, trailing: false }, Code([35, 18])], need_deref: true }");
    }
}
