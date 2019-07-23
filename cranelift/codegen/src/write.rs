//! Converting Cranelift IR to text.
//!
//! The `write` module provides the `write_function` function which converts an IR `Function` to an
//! equivalent textual form. This textual form can be read back by the `cranelift-reader` crate.

use crate::entity::SecondaryMap;
use crate::ir::entities::AnyEntity;
use crate::ir::immediates::Uimm128;
use crate::ir::{
    DataFlowGraph, DisplayFunctionAnnotations, Ebb, Function, Inst, SigRef, Type, Value, ValueDef,
    ValueLoc,
};
use crate::isa::{RegInfo, TargetIsa};
use crate::packed_option::ReservedValue;
use crate::value_label::ValueLabelsRanges;
use core::fmt::{self, Write};
use std::collections::HashSet;
use std::string::String;
use std::vec::Vec;

/// A `FuncWriter` used to decorate functions during printing.
pub trait FuncWriter {
    /// Write the extended basic block header for the current function.
    fn write_ebb_header(
        &mut self,
        w: &mut dyn Write,
        func: &Function,
        isa: Option<&dyn TargetIsa>,
        ebb: Ebb,
        indent: usize,
    ) -> fmt::Result;

    /// Write the given `inst` to `w`.
    fn write_instruction(
        &mut self,
        w: &mut dyn Write,
        func: &Function,
        aliases: &SecondaryMap<Value, Vec<Value>>,
        isa: Option<&dyn TargetIsa>,
        inst: Inst,
        indent: usize,
    ) -> fmt::Result;

    /// Write the preamble to `w`. By default, this uses `write_entity_definition`.
    fn write_preamble(
        &mut self,
        w: &mut dyn Write,
        func: &Function,
        regs: Option<&RegInfo>,
    ) -> Result<bool, fmt::Error> {
        self.super_preamble(w, func, regs)
    }

    /// Default impl of `write_preamble`
    fn super_preamble(
        &mut self,
        w: &mut dyn Write,
        func: &Function,
        regs: Option<&RegInfo>,
    ) -> Result<bool, fmt::Error> {
        let mut any = false;

        for (ss, slot) in func.stack_slots.iter() {
            any = true;
            self.write_entity_definition(w, func, ss.into(), slot)?;
        }

        for (gv, gv_data) in &func.global_values {
            any = true;
            self.write_entity_definition(w, func, gv.into(), gv_data)?;
        }

        for (heap, heap_data) in &func.heaps {
            if !heap_data.index_type.is_invalid() {
                any = true;
                self.write_entity_definition(w, func, heap.into(), heap_data)?;
            }
        }

        for (table, table_data) in &func.tables {
            if !table_data.index_type.is_invalid() {
                any = true;
                self.write_entity_definition(w, func, table.into(), table_data)?;
            }
        }

        // Write out all signatures before functions since function declarations can refer to
        // signatures.
        for (sig, sig_data) in &func.dfg.signatures {
            any = true;
            self.write_entity_definition(w, func, sig.into(), &sig_data.display(regs))?;
        }

        for (fnref, ext_func) in &func.dfg.ext_funcs {
            if ext_func.signature != SigRef::reserved_value() {
                any = true;
                self.write_entity_definition(w, func, fnref.into(), ext_func)?;
            }
        }

        for (jt, jt_data) in &func.jump_tables {
            any = true;
            self.write_entity_definition(w, func, jt.into(), jt_data)?;
        }

        Ok(any)
    }

    /// Write an entity definition defined in the preamble to `w`.
    fn write_entity_definition(
        &mut self,
        w: &mut dyn Write,
        func: &Function,
        entity: AnyEntity,
        value: &dyn fmt::Display,
    ) -> fmt::Result {
        self.super_entity_definition(w, func, entity, value)
    }

    /// Default impl of `write_entity_definition`
    #[allow(unused_variables)]
    fn super_entity_definition(
        &mut self,
        w: &mut dyn Write,
        func: &Function,
        entity: AnyEntity,
        value: &dyn fmt::Display,
    ) -> fmt::Result {
        writeln!(w, "    {} = {}", entity, value)
    }
}

/// A `PlainWriter` that doesn't decorate the function.
pub struct PlainWriter;

impl FuncWriter for PlainWriter {
    fn write_instruction(
        &mut self,
        w: &mut dyn Write,
        func: &Function,
        aliases: &SecondaryMap<Value, Vec<Value>>,
        isa: Option<&dyn TargetIsa>,
        inst: Inst,
        indent: usize,
    ) -> fmt::Result {
        write_instruction(w, func, aliases, isa, inst, indent)
    }

    fn write_ebb_header(
        &mut self,
        w: &mut dyn Write,
        func: &Function,
        isa: Option<&dyn TargetIsa>,
        ebb: Ebb,
        indent: usize,
    ) -> fmt::Result {
        write_ebb_header(w, func, isa, ebb, indent)
    }
}

/// Write `func` to `w` as equivalent text.
/// Use `isa` to emit ISA-dependent annotations.
pub fn write_function(
    w: &mut dyn Write,
    func: &Function,
    annotations: &DisplayFunctionAnnotations,
) -> fmt::Result {
    decorate_function(&mut PlainWriter, w, func, annotations)
}

/// Create a reverse-alias map from a value to all aliases having that value as a direct target
fn alias_map(func: &Function) -> SecondaryMap<Value, Vec<Value>> {
    let mut aliases = SecondaryMap::<_, Vec<_>>::new();
    for v in func.dfg.values() {
        // VADFS returns the immediate target of an alias
        if let Some(k) = func.dfg.value_alias_dest_for_serialization(v) {
            aliases[k].push(v);
        }
    }
    aliases
}

/// Writes `func` to `w` as text.
/// write_function_plain is passed as 'closure' to print instructions as text.
/// pretty_function_error is passed as 'closure' to add error decoration.
pub fn decorate_function<FW: FuncWriter>(
    func_w: &mut FW,
    w: &mut dyn Write,
    func: &Function,
    annotations: &DisplayFunctionAnnotations,
) -> fmt::Result {
    let regs = annotations.isa.map(TargetIsa::register_info);
    let regs = regs.as_ref();

    write!(w, "function ")?;
    write_spec(w, func, regs)?;
    writeln!(w, " {{")?;
    let aliases = alias_map(func);
    let mut any = func_w.write_preamble(w, func, regs)?;
    for ebb in &func.layout {
        if any {
            writeln!(w)?;
        }
        decorate_ebb(func_w, w, func, &aliases, annotations, ebb)?;
        any = true;
    }
    writeln!(w, "}}")
}

//----------------------------------------------------------------------
//
// Function spec.

fn write_spec(w: &mut dyn Write, func: &Function, regs: Option<&RegInfo>) -> fmt::Result {
    write!(w, "{}{}", func.name, func.signature.display(regs))
}

//----------------------------------------------------------------------
//
// Basic blocks

fn write_arg(
    w: &mut dyn Write,
    func: &Function,
    regs: Option<&RegInfo>,
    arg: Value,
) -> fmt::Result {
    write!(w, "{}: {}", arg, func.dfg.value_type(arg))?;
    let loc = func.locations[arg];
    if loc.is_assigned() {
        write!(w, " [{}]", loc.display(regs))?
    }

    Ok(())
}

/// Write out the basic block header, outdented:
///
///    ebb1:
///    ebb1(v1: i32):
///    ebb10(v4: f64, v5: b1):
///
pub fn write_ebb_header(
    w: &mut dyn Write,
    func: &Function,
    isa: Option<&dyn TargetIsa>,
    ebb: Ebb,
    indent: usize,
) -> fmt::Result {
    // The `indent` is the instruction indentation. EBB headers are 4 spaces out from that.
    write!(w, "{1:0$}{2}", indent - 4, "", ebb)?;

    let regs = isa.map(TargetIsa::register_info);
    let regs = regs.as_ref();

    let mut args = func.dfg.ebb_params(ebb).iter().cloned();
    match args.next() {
        None => return writeln!(w, ":"),
        Some(arg) => {
            write!(w, "(")?;
            write_arg(w, func, regs, arg)?;
        }
    }
    // Remaining arguments.
    for arg in args {
        write!(w, ", ")?;
        write_arg(w, func, regs, arg)?;
    }
    writeln!(w, "):")
}

fn write_valueloc(w: &mut dyn Write, loc: &ValueLoc, regs: &RegInfo) -> fmt::Result {
    match loc {
        ValueLoc::Reg(r) => write!(w, "{}", regs.display_regunit(*r)),
        ValueLoc::Stack(ss) => write!(w, "{}", ss),
        ValueLoc::Unassigned => write!(w, "?"),
    }
}

fn write_value_range_markers(
    w: &mut dyn Write,
    val_ranges: &ValueLabelsRanges,
    regs: &RegInfo,
    offset: u32,
    indent: usize,
) -> fmt::Result {
    let mut result = String::new();
    let mut shown = HashSet::new();
    for (val, rng) in val_ranges {
        for i in (0..rng.len()).rev() {
            if rng[i].start == offset {
                write!(&mut result, " {}@", val)?;
                write_valueloc(&mut result, &rng[i].loc, regs)?;
                shown.insert(val);
                break;
            }
        }
    }
    for (val, rng) in val_ranges {
        for i in (0..rng.len()).rev() {
            if rng[i].end == offset && !shown.contains(val) {
                write!(&mut result, " {}\u{2620}", val)?;
                break;
            }
        }
    }
    if result.len() > 0 {
        writeln!(w, ";{1:0$}; {2}", indent + 24, "", result)?;
    }
    Ok(())
}

fn decorate_ebb<FW: FuncWriter>(
    func_w: &mut FW,
    w: &mut dyn Write,
    func: &Function,
    aliases: &SecondaryMap<Value, Vec<Value>>,
    annotations: &DisplayFunctionAnnotations,
    ebb: Ebb,
) -> fmt::Result {
    // Indent all instructions if any encodings are present.
    let indent = if func.encodings.is_empty() && func.srclocs.is_empty() {
        4
    } else {
        36
    };
    let isa = annotations.isa;

    func_w.write_ebb_header(w, func, isa, ebb, indent)?;
    for a in func.dfg.ebb_params(ebb).iter().cloned() {
        write_value_aliases(w, aliases, a, indent)?;
    }

    if isa.is_some() && !func.offsets.is_empty() {
        let encinfo = isa.unwrap().encoding_info();
        let regs = &isa.unwrap().register_info();
        for (offset, inst, size) in func.inst_offsets(ebb, &encinfo) {
            func_w.write_instruction(w, func, aliases, isa, inst, indent)?;
            if size > 0 {
                if let Some(val_ranges) = annotations.value_ranges {
                    write_value_range_markers(w, val_ranges, regs, offset + size, indent)?;
                }
            }
        }
    } else {
        for inst in func.layout.ebb_insts(ebb) {
            func_w.write_instruction(w, func, aliases, isa, inst, indent)?;
        }
    }

    Ok(())
}

//----------------------------------------------------------------------
//
// Instructions

// Should `inst` be printed with a type suffix?
//
// Polymorphic instructions may need a suffix indicating the value of the controlling type variable
// if it can't be trivially inferred.
//
fn type_suffix(func: &Function, inst: Inst) -> Option<Type> {
    let inst_data = &func.dfg[inst];
    let constraints = inst_data.opcode().constraints();

    if !constraints.is_polymorphic() {
        return None;
    }

    // If the controlling type variable can be inferred from the type of the designated value input
    // operand, we don't need the type suffix.
    if constraints.use_typevar_operand() {
        let ctrl_var = inst_data.typevar_operand(&func.dfg.value_lists).unwrap();
        let def_ebb = match func.dfg.value_def(ctrl_var) {
            ValueDef::Result(instr, _) => func.layout.inst_ebb(instr),
            ValueDef::Param(ebb, _) => Some(ebb),
        };
        if def_ebb.is_some() && def_ebb == func.layout.inst_ebb(inst) {
            return None;
        }
    }

    let rtype = func.dfg.ctrl_typevar(inst);
    assert!(
        !rtype.is_invalid(),
        "Polymorphic instruction must produce a result"
    );
    Some(rtype)
}

/// Write out any aliases to the given target, including indirect aliases
fn write_value_aliases(
    w: &mut dyn Write,
    aliases: &SecondaryMap<Value, Vec<Value>>,
    target: Value,
    indent: usize,
) -> fmt::Result {
    let mut todo_stack = vec![target];
    while let Some(target) = todo_stack.pop() {
        for &a in &aliases[target] {
            writeln!(w, "{1:0$}{2} -> {3}", indent, "", a, target)?;
            todo_stack.push(a);
        }
    }

    Ok(())
}

fn write_instruction(
    w: &mut dyn Write,
    func: &Function,
    aliases: &SecondaryMap<Value, Vec<Value>>,
    isa: Option<&dyn TargetIsa>,
    inst: Inst,
    indent: usize,
) -> fmt::Result {
    // Prefix containing source location, encoding, and value locations.
    let mut s = String::with_capacity(16);

    // Source location goes first.
    let srcloc = func.srclocs[inst];
    if !srcloc.is_default() {
        write!(s, "{} ", srcloc)?;
    }

    // Write out encoding info.
    if let Some(enc) = func.encodings.get(inst).cloned() {
        if let Some(isa) = isa {
            write!(s, "[{}", isa.encoding_info().display(enc))?;
            // Write value locations, if we have them.
            if !func.locations.is_empty() {
                let regs = isa.register_info();
                for &r in func.dfg.inst_results(inst) {
                    write!(s, ",{}", func.locations[r].display(&regs))?
                }
            }
            write!(s, "] ")?;
        } else {
            write!(s, "[{}] ", enc)?;
        }
    }

    // Write out prefix and indent the instruction.
    write!(w, "{1:0$}", indent, s)?;

    // Write out the result values, if any.
    let mut has_results = false;
    for r in func.dfg.inst_results(inst) {
        if !has_results {
            has_results = true;
            write!(w, "{}", r)?;
        } else {
            write!(w, ", {}", r)?;
        }
    }
    if has_results {
        write!(w, " = ")?;
    }

    // Then the opcode, possibly with a '.type' suffix.
    let opcode = func.dfg[inst].opcode();

    match type_suffix(func, inst) {
        Some(suf) => write!(w, "{}.{}", opcode, suf)?,
        None => write!(w, "{}", opcode)?,
    }

    write_operands(w, &func.dfg, isa, inst)?;
    writeln!(w)?;

    // Value aliases come out on lines after the instruction defining the referent.
    for r in func.dfg.inst_results(inst) {
        write_value_aliases(w, aliases, *r, indent)?;
    }
    Ok(())
}

/// Write the operands of `inst` to `w` with a prepended space.
pub fn write_operands(
    w: &mut dyn Write,
    dfg: &DataFlowGraph,
    isa: Option<&dyn TargetIsa>,
    inst: Inst,
) -> fmt::Result {
    let pool = &dfg.value_lists;
    use crate::ir::instructions::InstructionData::*;
    match dfg[inst] {
        Unary { arg, .. } => write!(w, " {}", arg),
        UnaryImm { imm, .. } => write!(w, " {}", imm),
        UnaryImm128 { imm, .. } => {
            let data = dfg.constants.get(imm);
            let uimm128 = Uimm128::from(&data[..]);
            write!(w, " {}", uimm128)
        }
        UnaryIeee32 { imm, .. } => write!(w, " {}", imm),
        UnaryIeee64 { imm, .. } => write!(w, " {}", imm),
        UnaryBool { imm, .. } => write!(w, " {}", imm),
        UnaryGlobalValue { global_value, .. } => write!(w, " {}", global_value),
        Binary { args, .. } => write!(w, " {}, {}", args[0], args[1]),
        BinaryImm { arg, imm, .. } => write!(w, " {}, {}", arg, imm),
        Ternary { args, .. } => write!(w, " {}, {}, {}", args[0], args[1], args[2]),
        MultiAry { ref args, .. } => {
            if args.is_empty() {
                write!(w, "")
            } else {
                write!(w, " {}", DisplayValues(args.as_slice(pool)))
            }
        }
        NullAry { .. } => write!(w, " "),
        InsertLane { lane, args, .. } => write!(w, " {}, {}, {}", args[0], lane, args[1]),
        ExtractLane { lane, arg, .. } => write!(w, " {}, {}", arg, lane),
        IntCompare { cond, args, .. } => write!(w, " {} {}, {}", cond, args[0], args[1]),
        IntCompareImm { cond, arg, imm, .. } => write!(w, " {} {}, {}", cond, arg, imm),
        IntCond { cond, arg, .. } => write!(w, " {} {}", cond, arg),
        FloatCompare { cond, args, .. } => write!(w, " {} {}, {}", cond, args[0], args[1]),
        FloatCond { cond, arg, .. } => write!(w, " {} {}", cond, arg),
        IntSelect { cond, args, .. } => {
            write!(w, " {} {}, {}, {}", cond, args[0], args[1], args[2])
        }
        Jump {
            destination,
            ref args,
            ..
        } => {
            write!(w, " {}", destination)?;
            write_ebb_args(w, args.as_slice(pool))
        }
        Branch {
            destination,
            ref args,
            ..
        } => {
            let args = args.as_slice(pool);
            write!(w, " {}, {}", args[0], destination)?;
            write_ebb_args(w, &args[1..])
        }
        BranchInt {
            cond,
            destination,
            ref args,
            ..
        } => {
            let args = args.as_slice(pool);
            write!(w, " {} {}, {}", cond, args[0], destination)?;
            write_ebb_args(w, &args[1..])
        }
        BranchFloat {
            cond,
            destination,
            ref args,
            ..
        } => {
            let args = args.as_slice(pool);
            write!(w, " {} {}, {}", cond, args[0], destination)?;
            write_ebb_args(w, &args[1..])
        }
        BranchIcmp {
            cond,
            destination,
            ref args,
            ..
        } => {
            let args = args.as_slice(pool);
            write!(w, " {} {}, {}, {}", cond, args[0], args[1], destination)?;
            write_ebb_args(w, &args[2..])
        }
        BranchTable {
            arg,
            destination,
            table,
            ..
        } => write!(w, " {}, {}, {}", arg, destination, table),
        BranchTableBase { table, .. } => write!(w, " {}", table),
        BranchTableEntry {
            args, imm, table, ..
        } => write!(w, " {}, {}, {}, {}", args[0], args[1], imm, table),
        IndirectJump { arg, table, .. } => write!(w, " {}, {}", arg, table),
        Call {
            func_ref, ref args, ..
        } => write!(w, " {}({})", func_ref, DisplayValues(args.as_slice(pool))),
        CallIndirect {
            sig_ref, ref args, ..
        } => {
            let args = args.as_slice(pool);
            write!(
                w,
                " {}, {}({})",
                sig_ref,
                args[0],
                DisplayValues(&args[1..])
            )
        }
        FuncAddr { func_ref, .. } => write!(w, " {}", func_ref),
        StackLoad {
            stack_slot, offset, ..
        } => write!(w, " {}{}", stack_slot, offset),
        StackStore {
            arg,
            stack_slot,
            offset,
            ..
        } => write!(w, " {}, {}{}", arg, stack_slot, offset),
        HeapAddr { heap, arg, imm, .. } => write!(w, " {}, {}, {}", heap, arg, imm),
        TableAddr { table, arg, .. } => write!(w, " {}, {}", table, arg),
        Load {
            flags, arg, offset, ..
        } => write!(w, "{} {}{}", flags, arg, offset),
        LoadComplex {
            flags,
            ref args,
            offset,
            ..
        } => {
            let args = args.as_slice(pool);
            write!(
                w,
                "{} {}{}",
                flags,
                DisplayValuesWithDelimiter(&args, '+'),
                offset
            )
        }
        Store {
            flags,
            args,
            offset,
            ..
        } => write!(w, "{} {}, {}{}", flags, args[0], args[1], offset),
        StoreComplex {
            flags,
            ref args,
            offset,
            ..
        } => {
            let args = args.as_slice(pool);
            write!(
                w,
                "{} {}, {}{}",
                flags,
                args[0],
                DisplayValuesWithDelimiter(&args[1..], '+'),
                offset
            )
        }
        RegMove { arg, src, dst, .. } => {
            if let Some(isa) = isa {
                let regs = isa.register_info();
                write!(
                    w,
                    " {}, {} -> {}",
                    arg,
                    regs.display_regunit(src),
                    regs.display_regunit(dst)
                )
            } else {
                write!(w, " {}, %{} -> %{}", arg, src, dst)
            }
        }
        CopySpecial { src, dst, .. } => {
            if let Some(isa) = isa {
                let regs = isa.register_info();
                write!(
                    w,
                    " {} -> {}",
                    regs.display_regunit(src),
                    regs.display_regunit(dst)
                )
            } else {
                write!(w, " %{} -> %{}", src, dst)
            }
        }
        CopyToSsa { src, .. } => {
            if let Some(isa) = isa {
                let regs = isa.register_info();
                write!(w, " {}", regs.display_regunit(src))
            } else {
                write!(w, " %{}", src)
            }
        }
        RegSpill { arg, src, dst, .. } => {
            if let Some(isa) = isa {
                let regs = isa.register_info();
                write!(w, " {}, {} -> {}", arg, regs.display_regunit(src), dst)
            } else {
                write!(w, " {}, %{} -> {}", arg, src, dst)
            }
        }
        RegFill { arg, src, dst, .. } => {
            if let Some(isa) = isa {
                let regs = isa.register_info();
                write!(w, " {}, {} -> {}", arg, src, regs.display_regunit(dst))
            } else {
                write!(w, " {}, {} -> %{}", arg, src, dst)
            }
        }
        Trap { code, .. } => write!(w, " {}", code),
        CondTrap { arg, code, .. } => write!(w, " {}, {}", arg, code),
        IntCondTrap {
            cond, arg, code, ..
        } => write!(w, " {} {}, {}", cond, arg, code),
        FloatCondTrap {
            cond, arg, code, ..
        } => write!(w, " {} {}, {}", cond, arg, code),
    }
}

/// Write EBB args using optional parantheses.
fn write_ebb_args(w: &mut dyn Write, args: &[Value]) -> fmt::Result {
    if args.is_empty() {
        Ok(())
    } else {
        write!(w, "({})", DisplayValues(args))
    }
}

/// Displayable slice of values.
struct DisplayValues<'a>(&'a [Value]);

impl<'a> fmt::Display for DisplayValues<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, val) in self.0.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", val)?;
            } else {
                write!(f, ", {}", val)?;
            }
        }
        Ok(())
    }
}

struct DisplayValuesWithDelimiter<'a>(&'a [Value], char);

impl<'a> fmt::Display for DisplayValuesWithDelimiter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, val) in self.0.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", val)?;
            } else {
                write!(f, "{}{}", self.1, val)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::cursor::{Cursor, CursorPosition, FuncCursor};
    use crate::ir::types;
    use crate::ir::{ExternalName, Function, InstBuilder, StackSlotData, StackSlotKind};
    use std::string::ToString;

    #[test]
    fn basic() {
        let mut f = Function::new();
        assert_eq!(f.to_string(), "function u0:0() fast {\n}\n");

        f.name = ExternalName::testcase("foo");
        assert_eq!(f.to_string(), "function %foo() fast {\n}\n");

        f.create_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 4));
        assert_eq!(
            f.to_string(),
            "function %foo() fast {\n    ss0 = explicit_slot 4\n}\n"
        );

        let ebb = f.dfg.make_ebb();
        f.layout.append_ebb(ebb);
        assert_eq!(
            f.to_string(),
            "function %foo() fast {\n    ss0 = explicit_slot 4\n\nebb0:\n}\n"
        );

        f.dfg.append_ebb_param(ebb, types::I8);
        assert_eq!(
            f.to_string(),
            "function %foo() fast {\n    ss0 = explicit_slot 4\n\nebb0(v0: i8):\n}\n"
        );

        f.dfg.append_ebb_param(ebb, types::F32.by(4).unwrap());
        assert_eq!(
            f.to_string(),
            "function %foo() fast {\n    ss0 = explicit_slot 4\n\nebb0(v0: i8, v1: f32x4):\n}\n"
        );

        {
            let mut cursor = FuncCursor::new(&mut f);
            cursor.set_position(CursorPosition::After(ebb));
            cursor.ins().return_(&[])
        };
        assert_eq!(
            f.to_string(),
            "function %foo() fast {\n    ss0 = explicit_slot 4\n\nebb0(v0: i8, v1: f32x4):\n    return\n}\n"
        );
    }

    #[test]
    fn aliases() {
        use crate::ir::InstBuilder;

        let mut func = Function::new();
        {
            let ebb0 = func.dfg.make_ebb();
            let mut pos = FuncCursor::new(&mut func);
            pos.insert_ebb(ebb0);

            // make some detached values for change_to_alias
            let v0 = pos.func.dfg.append_ebb_param(ebb0, types::I32);
            let v1 = pos.func.dfg.append_ebb_param(ebb0, types::I32);
            let v2 = pos.func.dfg.append_ebb_param(ebb0, types::I32);
            pos.func.dfg.detach_ebb_params(ebb0);

            // alias to a param--will be printed at beginning of ebb defining param
            let v3 = pos.func.dfg.append_ebb_param(ebb0, types::I32);
            pos.func.dfg.change_to_alias(v0, v3);

            // alias to an alias--should print attached to alias, not ultimate target
            pos.func.dfg.make_value_alias_for_serialization(v0, v2); // v0 <- v2

            // alias to a result--will be printed after instruction producing result
            let _dummy0 = pos.ins().iconst(types::I32, 42);
            let v4 = pos.ins().iadd(v0, v0);
            pos.func.dfg.change_to_alias(v1, v4);
            let _dummy1 = pos.ins().iconst(types::I32, 23);
            let _v7 = pos.ins().iadd(v1, v1);
        }
        assert_eq!(
            func.to_string(),
            "function u0:0() fast {\nebb0(v3: i32):\n    v0 -> v3\n    v2 -> v0\n    v4 = iconst.i32 42\n    v5 = iadd v0, v0\n    v1 -> v5\n    v6 = iconst.i32 23\n    v7 = iadd v1, v1\n}\n"
        );
    }
}
