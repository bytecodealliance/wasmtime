use crate::abi::ABI;
use crate::codegen::CodeGen;
use crate::masm::{MacroAssembler, OperandSize, RegImm};
use crate::stack::Val;
use anyhow::Result;
use wasmparser::{FuncValidator, Result as ValidationResult, ValidatorResources, VisitOperator};
use wasmtime_environ::WasmType;

/// A macro iterator, inspired by wasmparsers `for_each_operator`,
/// containing the definition of supported operators
macro_rules! for_each_supported_op {
    ($def:ident) => {
        $def! {
            @mvp I32Const { value: i32 } => (visit_i32_const emit_i32_const)
            @mvp I32Add => (visit_i32_add emit_i32_add)
        @mvp LocalGet { local_index: u32 } => (visit_local_get emit_local_get)
        @mvp LocalSet { local_index: u32 } => (visit_local_set emit_local_set)
        }
    };
}

/// A macro iterator, inspired by wasmparsers `for_each_operator`,
/// containing the definition of unsupported operators
macro_rules! for_each_unsupported_op {
    ($def:ident) => {
	$def! {
	    @mvp Unreachable => visit_unreachable
	    @mvp Nop => visit_nop
	    @mvp Block { blockty: wasmparser::BlockType } => visit_block
	    @mvp Loop { blockty: wasmparser::BlockType } => visit_loop
	    @mvp If { blockty: wasmparser::BlockType } => visit_if
	    @mvp Else => visit_else
	    @exceptions Try { blockty: wasmparser::BlockType } => visit_try
	    @exceptions Catch { tag_index: u32 } => visit_catch
	    @exceptions Throw { tag_index: u32 } => visit_throw
	    @exceptions Rethrow { relative_depth: u32 } => visit_rethrow
	    // @mvp End => visit_end
	    @mvp Br { relative_depth: u32 } => visit_br
	    @mvp BrIf { relative_depth: u32 } => visit_br_if
	    @mvp BrTable { targets: wasmparser::BrTable<'a> } => visit_br_table
	    @mvp Return => visit_return
	    @mvp Call { function_index: u32 } => visit_call
	    @mvp CallIndirect { type_index: u32, table_index: u32, table_byte: u8 } => visit_call_indirect
	    @tail_calls ReturnCall { function_index: u32 } => visit_return_call
	    @tail_calls ReturnCallIndirect { type_index: u32, table_index: u32 } => visit_return_call_indirect
	    @exceptions Delegate { relative_depth: u32 } => visit_delegate
	    @exceptions CatchAll => visit_catch_all
	    @mvp Drop => visit_drop
	    @mvp Select => visit_select
	    @reference_types TypedSelect { ty: wasmparser::ValType } => visit_typed_select
	    @mvp LocalTee { local_index: u32 } => visit_local_tee
	    @mvp GlobalGet { global_index: u32 } => visit_global_get
	    @mvp GlobalSet { global_index: u32 } => visit_global_set
	    @mvp I32Load { memarg: wasmparser::MemArg } => visit_i32_load
	    @mvp I64Load { memarg: wasmparser::MemArg } => visit_i64_load
	    @mvp F32Load { memarg: wasmparser::MemArg } => visit_f32_load
	    @mvp F64Load { memarg: wasmparser::MemArg } => visit_f64_load
	    @mvp I32Load8S { memarg: wasmparser::MemArg } => visit_i32_load8_s
	    @mvp I32Load8U { memarg: wasmparser::MemArg } => visit_i32_load8_u
	    @mvp I32Load16S { memarg: wasmparser::MemArg } => visit_i32_load16_s
	    @mvp I32Load16U { memarg: wasmparser::MemArg } => visit_i32_load16_u
	    @mvp I64Load8S { memarg: wasmparser::MemArg } => visit_i64_load8_s
	    @mvp I64Load8U { memarg: wasmparser::MemArg } => visit_i64_load8_u
	    @mvp I64Load16S { memarg: wasmparser::MemArg } => visit_i64_load16_s
	    @mvp I64Load16U { memarg: wasmparser::MemArg } => visit_i64_load16_u
	    @mvp I64Load32S { memarg: wasmparser::MemArg } => visit_i64_load32_s
	    @mvp I64Load32U { memarg: wasmparser::MemArg } => visit_i64_load32_u
	    @mvp I32Store { memarg: wasmparser::MemArg } => visit_i32_store
	    @mvp I64Store { memarg: wasmparser::MemArg } => visit_i64_store
	    @mvp F32Store { memarg: wasmparser::MemArg } => visit_f32_store
	    @mvp F64Store { memarg: wasmparser::MemArg } => visit_f64_store
	    @mvp I32Store8 { memarg: wasmparser::MemArg } => visit_i32_store8
	    @mvp I32Store16 { memarg: wasmparser::MemArg } => visit_i32_store16
	    @mvp I64Store8 { memarg: wasmparser::MemArg } => visit_i64_store8
	    @mvp I64Store16 { memarg: wasmparser::MemArg } => visit_i64_store16
	    @mvp I64Store32 { memarg: wasmparser::MemArg } => visit_i64_store32
	    @mvp MemorySize { mem: u32, mem_byte: u8 } => visit_memory_size
	    @mvp MemoryGrow { mem: u32, mem_byte: u8 } => visit_memory_grow
	    @mvp I64Const { value: i64 } => visit_i64_const
	    @mvp F32Const { value: wasmparser::Ieee32 } => visit_f32_const
	    @mvp F64Const { value: wasmparser::Ieee64 } => visit_f64_const
	    @reference_types RefNull { ty: wasmparser::ValType } => visit_ref_null
	    @reference_types RefIsNull => visit_ref_is_null
	    @reference_types RefFunc { function_index: u32 } => visit_ref_func
	    @mvp I32Eqz => visit_i32_eqz
	    @mvp I32Eq => visit_i32_eq
	    @mvp I32Ne => visit_i32_ne
	    @mvp I32LtS => visit_i32_lt_s
	    @mvp I32LtU => visit_i32_lt_u
	    @mvp I32GtS => visit_i32_gt_s
	    @mvp I32GtU => visit_i32_gt_u
	    @mvp I32LeS => visit_i32_le_s
	    @mvp I32LeU => visit_i32_le_u
	    @mvp I32GeS => visit_i32_ge_s
	    @mvp I32GeU => visit_i32_ge_u
	    @mvp I64Eqz => visit_i64_eqz
	    @mvp I64Eq => visit_i64_eq
	    @mvp I64Ne => visit_i64_ne
	    @mvp I64LtS => visit_i64_lt_s
	    @mvp I64LtU => visit_i64_lt_u
	    @mvp I64GtS => visit_i64_gt_s
	    @mvp I64GtU => visit_i64_gt_u
	    @mvp I64LeS => visit_i64_le_s
	    @mvp I64LeU => visit_i64_le_u
	    @mvp I64GeS => visit_i64_ge_s
	    @mvp I64GeU => visit_i64_ge_u
	    @mvp F32Eq => visit_f32_eq
	    @mvp F32Ne => visit_f32_ne
	    @mvp F32Lt => visit_f32_lt
	    @mvp F32Gt => visit_f32_gt
	    @mvp F32Le => visit_f32_le
	    @mvp F32Ge => visit_f32_ge
	    @mvp F64Eq => visit_f64_eq
	    @mvp F64Ne => visit_f64_ne
	    @mvp F64Lt => visit_f64_lt
	    @mvp F64Gt => visit_f64_gt
	    @mvp F64Le => visit_f64_le
	    @mvp F64Ge => visit_f64_ge
	    @mvp I32Clz => visit_i32_clz
	    @mvp I32Ctz => visit_i32_ctz
	    @mvp I32Popcnt => visit_i32_popcnt
	    @mvp I32Sub => visit_i32_sub
	    @mvp I32Mul => visit_i32_mul
	    @mvp I32DivS => visit_i32_div_s
	    @mvp I32DivU => visit_i32_div_u
	    @mvp I32RemS => visit_i32_rem_s
	    @mvp I32RemU => visit_i32_rem_u
	    @mvp I32And => visit_i32_and
	    @mvp I32Or => visit_i32_or
	    @mvp I32Xor => visit_i32_xor
	    @mvp I32Shl => visit_i32_shl
	    @mvp I32ShrS => visit_i32_shr_s
	    @mvp I32ShrU => visit_i32_shr_u
	    @mvp I32Rotl => visit_i32_rotl
	    @mvp I32Rotr => visit_i32_rotr
	    @mvp I64Clz => visit_i64_clz
	    @mvp I64Ctz => visit_i64_ctz
	    @mvp I64Popcnt => visit_i64_popcnt
	    @mvp I64Add => visit_i64_add
	    @mvp I64Sub => visit_i64_sub
	    @mvp I64Mul => visit_i64_mul
	    @mvp I64DivS => visit_i64_div_s
	    @mvp I64DivU => visit_i64_div_u
	    @mvp I64RemS => visit_i64_rem_s
	    @mvp I64RemU => visit_i64_rem_u
	    @mvp I64And => visit_i64_and
	    @mvp I64Or => visit_i64_or
	    @mvp I64Xor => visit_i64_xor
	    @mvp I64Shl => visit_i64_shl
	    @mvp I64ShrS => visit_i64_shr_s
	    @mvp I64ShrU => visit_i64_shr_u
	    @mvp I64Rotl => visit_i64_rotl
	    @mvp I64Rotr => visit_i64_rotr
	    @mvp F32Abs => visit_f32_abs
	    @mvp F32Neg => visit_f32_neg
	    @mvp F32Ceil => visit_f32_ceil
	    @mvp F32Floor => visit_f32_floor
	    @mvp F32Trunc => visit_f32_trunc
	    @mvp F32Nearest => visit_f32_nearest
	    @mvp F32Sqrt => visit_f32_sqrt
	    @mvp F32Add => visit_f32_add
	    @mvp F32Sub => visit_f32_sub
	    @mvp F32Mul => visit_f32_mul
	    @mvp F32Div => visit_f32_div
	    @mvp F32Min => visit_f32_min
	    @mvp F32Max => visit_f32_max
	    @mvp F32Copysign => visit_f32_copysign
	    @mvp F64Abs => visit_f64_abs
	    @mvp F64Neg => visit_f64_neg
	    @mvp F64Ceil => visit_f64_ceil
	    @mvp F64Floor => visit_f64_floor
	    @mvp F64Trunc => visit_f64_trunc
	    @mvp F64Nearest => visit_f64_nearest
	    @mvp F64Sqrt => visit_f64_sqrt
	    @mvp F64Add => visit_f64_add
	    @mvp F64Sub => visit_f64_sub
	    @mvp F64Mul => visit_f64_mul
	    @mvp F64Div => visit_f64_div
	    @mvp F64Min => visit_f64_min
	    @mvp F64Max => visit_f64_max
	    @mvp F64Copysign => visit_f64_copysign
	    @mvp I32WrapI64 => visit_i32_wrap_i64
	    @mvp I32TruncF32S => visit_i32_trunc_f32_s
	    @mvp I32TruncF32U => visit_i32_trunc_f32_u
	    @mvp I32TruncF64S => visit_i32_trunc_f64_s
	    @mvp I32TruncF64U => visit_i32_trunc_f64_u
	    @mvp I64ExtendI32S => visit_i64_extend_i32_s
	    @mvp I64ExtendI32U => visit_i64_extend_i32_u
	    @mvp I64TruncF32S => visit_i64_trunc_f32_s
	    @mvp I64TruncF32U => visit_i64_trunc_f32_u
	    @mvp I64TruncF64S => visit_i64_trunc_f64_s
	    @mvp I64TruncF64U => visit_i64_trunc_f64_u
	    @mvp F32ConvertI32S => visit_f32_convert_i32_s
	    @mvp F32ConvertI32U => visit_f32_convert_i32_u
	    @mvp F32ConvertI64S => visit_f32_convert_i64_s
	    @mvp F32ConvertI64U => visit_f32_convert_i64_u
	    @mvp F32DemoteF64 => visit_f32_demote_f64
	    @mvp F64ConvertI32S => visit_f64_convert_i32_s
	    @mvp F64ConvertI32U => visit_f64_convert_i32_u
	    @mvp F64ConvertI64S => visit_f64_convert_i64_s
	    @mvp F64ConvertI64U => visit_f64_convert_i64_u
	    @mvp F64PromoteF32 => visit_f64_promote_f32
	    @mvp I32ReinterpretF32 => visit_i32_reinterpret_f32
	    @mvp I64ReinterpretF64 => visit_i64_reinterpret_f64
	    @mvp F32ReinterpretI32 => visit_f32_reinterpret_i32
	    @mvp F64ReinterpretI64 => visit_f64_reinterpret_i64
	    @sign_ext_ops I32Extend8S => visit_i32_extend8_s
	    @sign_ext_ops I32Extend16S => visit_i32_extend16_s
	    @sign_ext_ops I64Extend8S => visit_i64_extend8_s
	    @sign_ext_ops I64Extend16S => visit_i64_extend16_s
	    @sign_ext_ops I64Extend32S => visit_i64_extend32_s

	    // 0xFC operators
	    // Non-trapping Float-to-int Conversions
	    @non_trapping_f2i_conversions I32TruncSatF32S => visit_i32_trunc_sat_f32_s
	    @non_trapping_f2i_conversions I32TruncSatF32U => visit_i32_trunc_sat_f32_u
	    @non_trapping_f2i_conversions I32TruncSatF64S => visit_i32_trunc_sat_f64_s
	    @non_trapping_f2i_conversions I32TruncSatF64U => visit_i32_trunc_sat_f64_u
	    @non_trapping_f2i_conversions I64TruncSatF32S => visit_i64_trunc_sat_f32_s
	    @non_trapping_f2i_conversions I64TruncSatF32U => visit_i64_trunc_sat_f32_u
	    @non_trapping_f2i_conversions I64TruncSatF64S => visit_i64_trunc_sat_f64_s
	    @non_trapping_f2i_conversions I64TruncSatF64U => visit_i64_trunc_sat_f64_u

	    // 0xFC operators
	    // bulk memory https://github.com/WebAssembly/bulk-memory-operations/blob/master/proposals/bulk-memory-operations/Overview.md
	    @bulk_memory MemoryInit { data_index: u32, mem: u32 } => visit_memory_init
	    @bulk_memory DataDrop { data_index: u32 } => visit_data_drop
	    @bulk_memory MemoryCopy { dst_mem: u32, src_mem: u32 } => visit_memory_copy
	    @bulk_memory MemoryFill { mem: u32 } => visit_memory_fill
	    @bulk_memory TableInit { elem_index: u32, table: u32 } => visit_table_init
	    @bulk_memory ElemDrop { elem_index: u32 } => visit_elem_drop
	    @bulk_memory TableCopy { dst_table: u32, src_table: u32 } => visit_table_copy
	    @bulk_memory TableFill { table: u32 } => visit_table_fill
	    @bulk_memory TableGet { table: u32 } => visit_table_get
	    @bulk_memory TableSet { table: u32 } => visit_table_set
	    @bulk_memory TableGrow { table: u32 } => visit_table_grow
	    @bulk_memory TableSize { table: u32 } => visit_table_size

	    // 0xFE operators
	    // https://github.com/WebAssembly/threads/blob/master/proposals/threads/Overview.md
	    @threads MemoryAtomicNotify { memarg: wasmparser::MemArg } => visit_memory_atomic_notify
	    @threads MemoryAtomicWait32 { memarg: wasmparser::MemArg } => visit_memory_atomic_wait32
	    @threads MemoryAtomicWait64 { memarg: wasmparser::MemArg } => visit_memory_atomic_wait64
	    @threads AtomicFence => visit_atomic_fence
	    @threads I32AtomicLoad { memarg: wasmparser::MemArg } => visit_i32_atomic_load
	    @threads I64AtomicLoad { memarg: wasmparser::MemArg } => visit_i64_atomic_load
	    @threads I32AtomicLoad8U { memarg: wasmparser::MemArg } => visit_i32_atomic_load8_u
	    @threads I32AtomicLoad16U { memarg: wasmparser::MemArg } => visit_i32_atomic_load16_u
	    @threads I64AtomicLoad8U { memarg: wasmparser::MemArg } => visit_i64_atomic_load8_u
	    @threads I64AtomicLoad16U { memarg: wasmparser::MemArg } => visit_i64_atomic_load16_u
	    @threads I64AtomicLoad32U { memarg: wasmparser::MemArg } => visit_i64_atomic_load32_u
	    @threads I32AtomicStore { memarg: wasmparser::MemArg } => visit_i32_atomic_store
	    @threads I64AtomicStore { memarg: wasmparser::MemArg } => visit_i64_atomic_store
	    @threads I32AtomicStore8 { memarg: wasmparser::MemArg } => visit_i32_atomic_store8
	    @threads I32AtomicStore16 { memarg: wasmparser::MemArg } => visit_i32_atomic_store16
	    @threads I64AtomicStore8 { memarg: wasmparser::MemArg } => visit_i64_atomic_store8
	    @threads I64AtomicStore16 { memarg: wasmparser::MemArg } => visit_i64_atomic_store16
	    @threads I64AtomicStore32 { memarg: wasmparser::MemArg } => visit_i64_atomic_store32
	    @threads I32AtomicRmwAdd { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw_add
	    @threads I64AtomicRmwAdd { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw_add
	    @threads I32AtomicRmw8AddU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw8_add_u
	    @threads I32AtomicRmw16AddU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw16_add_u
	    @threads I64AtomicRmw8AddU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw8_add_u
	    @threads I64AtomicRmw16AddU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw16_add_u
	    @threads I64AtomicRmw32AddU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw32_add_u
	    @threads I32AtomicRmwSub { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw_sub
	    @threads I64AtomicRmwSub { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw_sub
	    @threads I32AtomicRmw8SubU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw8_sub_u
	    @threads I32AtomicRmw16SubU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw16_sub_u
	    @threads I64AtomicRmw8SubU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw8_sub_u
	    @threads I64AtomicRmw16SubU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw16_sub_u
	    @threads I64AtomicRmw32SubU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw32_sub_u
	    @threads I32AtomicRmwAnd { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw_and
	    @threads I64AtomicRmwAnd { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw_and
	    @threads I32AtomicRmw8AndU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw8_and_u
	    @threads I32AtomicRmw16AndU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw16_and_u
	    @threads I64AtomicRmw8AndU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw8_and_u
	    @threads I64AtomicRmw16AndU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw16_and_u
	    @threads I64AtomicRmw32AndU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw32_and_u
	    @threads I32AtomicRmwOr { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw_or
	    @threads I64AtomicRmwOr { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw_or
	    @threads I32AtomicRmw8OrU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw8_or_u
	    @threads I32AtomicRmw16OrU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw16_or_u
	    @threads I64AtomicRmw8OrU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw8_or_u
	    @threads I64AtomicRmw16OrU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw16_or_u
	    @threads I64AtomicRmw32OrU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw32_or_u
	    @threads I32AtomicRmwXor { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw_xor
	    @threads I64AtomicRmwXor { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw_xor
	    @threads I32AtomicRmw8XorU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw8_xor_u
	    @threads I32AtomicRmw16XorU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw16_xor_u
	    @threads I64AtomicRmw8XorU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw8_xor_u
	    @threads I64AtomicRmw16XorU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw16_xor_u
	    @threads I64AtomicRmw32XorU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw32_xor_u
	    @threads I32AtomicRmwXchg { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw_xchg
	    @threads I64AtomicRmwXchg { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw_xchg
	    @threads I32AtomicRmw8XchgU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw8_xchg_u
	    @threads I32AtomicRmw16XchgU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw16_xchg_u
	    @threads I64AtomicRmw8XchgU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw8_xchg_u
	    @threads I64AtomicRmw16XchgU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw16_xchg_u
	    @threads I64AtomicRmw32XchgU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw32_xchg_u
	    @threads I32AtomicRmwCmpxchg { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw_cmpxchg
	    @threads I64AtomicRmwCmpxchg { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw_cmpxchg
	    @threads I32AtomicRmw8CmpxchgU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw8_cmpxchg_u
	    @threads I32AtomicRmw16CmpxchgU { memarg: wasmparser::MemArg } => visit_i32_atomic_rmw16_cmpxchg_u
	    @threads I64AtomicRmw8CmpxchgU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw8_cmpxchg_u
	    @threads I64AtomicRmw16CmpxchgU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw16_cmpxchg_u
	    @threads I64AtomicRmw32CmpxchgU { memarg: wasmparser::MemArg } => visit_i64_atomic_rmw32_cmpxchg_u

	    // 0xFD operators
	    // SIMD https://webassembly.github.io/simd/core/binary/instructions.html
	    @simd V128Load { memarg: wasmparser::MemArg } => visit_v128_load
	    @simd V128Load8x8S { memarg: wasmparser::MemArg } => visit_v128_load8x8_s
	    @simd V128Load8x8U { memarg: wasmparser::MemArg } => visit_v128_load8x8_u
	    @simd V128Load16x4S { memarg: wasmparser::MemArg } => visit_v128_load16x4_s
	    @simd V128Load16x4U { memarg: wasmparser::MemArg } => visit_v128_load16x4_u
	    @simd V128Load32x2S { memarg: wasmparser::MemArg } => visit_v128_load32x2_s
	    @simd V128Load32x2U { memarg: wasmparser::MemArg } => visit_v128_load32x2_u
	    @simd V128Load8Splat { memarg: wasmparser::MemArg } => visit_v128_load8_splat
	    @simd V128Load16Splat { memarg: wasmparser::MemArg } => visit_v128_load16_splat
	    @simd V128Load32Splat { memarg: wasmparser::MemArg } => visit_v128_load32_splat
	    @simd V128Load64Splat { memarg: wasmparser::MemArg } => visit_v128_load64_splat
	    @simd V128Load32Zero { memarg: wasmparser::MemArg } => visit_v128_load32_zero
	    @simd V128Load64Zero { memarg: wasmparser::MemArg } => visit_v128_load64_zero
	    @simd V128Store { memarg: wasmparser::MemArg } => visit_v128_store
	    @simd V128Load8Lane { memarg: wasmparser::MemArg, lane: u8 } => visit_v128_load8_lane
	    @simd V128Load16Lane { memarg: wasmparser::MemArg, lane: u8 } => visit_v128_load16_lane
	    @simd V128Load32Lane { memarg: wasmparser::MemArg, lane: u8 } => visit_v128_load32_lane
	    @simd V128Load64Lane { memarg: wasmparser::MemArg, lane: u8 } => visit_v128_load64_lane
	    @simd V128Store8Lane { memarg: wasmparser::MemArg, lane: u8 } => visit_v128_store8_lane
	    @simd V128Store16Lane { memarg: wasmparser::MemArg, lane: u8 } => visit_v128_store16_lane
	    @simd V128Store32Lane { memarg: wasmparser::MemArg, lane: u8 } => visit_v128_store32_lane
	    @simd V128Store64Lane { memarg: wasmparser::MemArg, lane: u8 } => visit_v128_store64_lane
	    @simd V128Const { value: wasmparser::V128 } => visit_v128_const
	    @simd I8x16Shuffle { lanes: [u8; 16] } => visit_i8x16_shuffle
	    @simd I8x16ExtractLaneS { lane: u8 } => visit_i8x16_extract_lane_s
	    @simd I8x16ExtractLaneU { lane: u8 } => visit_i8x16_extract_lane_u
	    @simd I8x16ReplaceLane { lane: u8 } => visit_i8x16_replace_lane
	    @simd I16x8ExtractLaneS { lane: u8 } => visit_i16x8_extract_lane_s
	    @simd I16x8ExtractLaneU { lane: u8 } => visit_i16x8_extract_lane_u
	    @simd I16x8ReplaceLane { lane: u8 } => visit_i16x8_replace_lane
	    @simd I32x4ExtractLane { lane: u8 } => visit_i32x4_extract_lane
	    @simd I32x4ReplaceLane { lane: u8 } => visit_i32x4_replace_lane
	    @simd I64x2ExtractLane { lane: u8 } => visit_i64x2_extract_lane
	    @simd I64x2ReplaceLane { lane: u8 } => visit_i64x2_replace_lane
	    @simd F32x4ExtractLane { lane: u8 } => visit_f32x4_extract_lane
	    @simd F32x4ReplaceLane { lane: u8 } => visit_f32x4_replace_lane
	    @simd F64x2ExtractLane { lane: u8 } => visit_f64x2_extract_lane
	    @simd F64x2ReplaceLane { lane: u8 } => visit_f64x2_replace_lane
	    @simd I8x16Swizzle => visit_i8x16_swizzle
	    @simd I8x16Splat => visit_i8x16_splat
	    @simd I16x8Splat => visit_i16x8_splat
	    @simd I32x4Splat => visit_i32x4_splat
	    @simd I64x2Splat => visit_i64x2_splat
	    @simd F32x4Splat => visit_f32x4_splat
	    @simd F64x2Splat => visit_f64x2_splat
	    @simd I8x16Eq => visit_i8x16_eq
	    @simd I8x16Ne => visit_i8x16_ne
	    @simd I8x16LtS => visit_i8x16_lt_s
	    @simd I8x16LtU => visit_i8x16_lt_u
	    @simd I8x16GtS => visit_i8x16_gt_s
	    @simd I8x16GtU => visit_i8x16_gt_u
	    @simd I8x16LeS => visit_i8x16_le_s
	    @simd I8x16LeU => visit_i8x16_le_u
	    @simd I8x16GeS => visit_i8x16_ge_s
	    @simd I8x16GeU => visit_i8x16_ge_u
	    @simd I16x8Eq => visit_i16x8_eq
	    @simd I16x8Ne => visit_i16x8_ne
	    @simd I16x8LtS => visit_i16x8_lt_s
	    @simd I16x8LtU => visit_i16x8_lt_u
	    @simd I16x8GtS => visit_i16x8_gt_s
	    @simd I16x8GtU => visit_i16x8_gt_u
	    @simd I16x8LeS => visit_i16x8_le_s
	    @simd I16x8LeU => visit_i16x8_le_u
	    @simd I16x8GeS => visit_i16x8_ge_s
	    @simd I16x8GeU => visit_i16x8_ge_u
	    @simd I32x4Eq => visit_i32x4_eq
	    @simd I32x4Ne => visit_i32x4_ne
	    @simd I32x4LtS => visit_i32x4_lt_s
	    @simd I32x4LtU => visit_i32x4_lt_u
	    @simd I32x4GtS => visit_i32x4_gt_s
	    @simd I32x4GtU => visit_i32x4_gt_u
	    @simd I32x4LeS => visit_i32x4_le_s
	    @simd I32x4LeU => visit_i32x4_le_u
	    @simd I32x4GeS => visit_i32x4_ge_s
	    @simd I32x4GeU => visit_i32x4_ge_u
	    @simd I64x2Eq => visit_i64x2_eq
	    @simd I64x2Ne => visit_i64x2_ne
	    @simd I64x2LtS => visit_i64x2_lt_s
	    @simd I64x2GtS => visit_i64x2_gt_s
	    @simd I64x2LeS => visit_i64x2_le_s
	    @simd I64x2GeS => visit_i64x2_ge_s
	    @simd F32x4Eq => visit_f32x4_eq
	    @simd F32x4Ne => visit_f32x4_ne
	    @simd F32x4Lt => visit_f32x4_lt
	    @simd F32x4Gt => visit_f32x4_gt
	    @simd F32x4Le => visit_f32x4_le
	    @simd F32x4Ge => visit_f32x4_ge
	    @simd F64x2Eq => visit_f64x2_eq
	    @simd F64x2Ne => visit_f64x2_ne
	    @simd F64x2Lt => visit_f64x2_lt
	    @simd F64x2Gt => visit_f64x2_gt
	    @simd F64x2Le => visit_f64x2_le
	    @simd F64x2Ge => visit_f64x2_ge
	    @simd V128Not => visit_v128_not
	    @simd V128And => visit_v128_and
	    @simd V128AndNot => visit_v128_andnot
	    @simd V128Or => visit_v128_or
	    @simd V128Xor => visit_v128_xor
	    @simd V128Bitselect => visit_v128_bitselect
	    @simd V128AnyTrue => visit_v128_any_true
	    @simd I8x16Abs => visit_i8x16_abs
	    @simd I8x16Neg => visit_i8x16_neg
	    @simd I8x16Popcnt => visit_i8x16_popcnt
	    @simd I8x16AllTrue => visit_i8x16_all_true
	    @simd I8x16Bitmask => visit_i8x16_bitmask
	    @simd I8x16NarrowI16x8S => visit_i8x16_narrow_i16x8_s
	    @simd I8x16NarrowI16x8U => visit_i8x16_narrow_i16x8_u
	    @simd I8x16Shl => visit_i8x16_shl
	    @simd I8x16ShrS => visit_i8x16_shr_s
	    @simd I8x16ShrU => visit_i8x16_shr_u
	    @simd I8x16Add => visit_i8x16_add
	    @simd I8x16AddSatS => visit_i8x16_add_sat_s
	    @simd I8x16AddSatU => visit_i8x16_add_sat_u
	    @simd I8x16Sub => visit_i8x16_sub
	    @simd I8x16SubSatS => visit_i8x16_sub_sat_s
	    @simd I8x16SubSatU => visit_i8x16_sub_sat_u
	    @simd I8x16MinS => visit_i8x16_min_s
	    @simd I8x16MinU => visit_i8x16_min_u
	    @simd I8x16MaxS => visit_i8x16_max_s
	    @simd I8x16MaxU => visit_i8x16_max_u
	    @simd I8x16RoundingAverageU => visit_i8x16_avgr_u
	    @simd I16x8ExtAddPairwiseI8x16S => visit_i16x8_extadd_pairwise_i8x16_s
	    @simd I16x8ExtAddPairwiseI8x16U => visit_i16x8_extadd_pairwise_i8x16_u
	    @simd I16x8Abs => visit_i16x8_abs
	    @simd I16x8Neg => visit_i16x8_neg
	    @simd I16x8Q15MulrSatS => visit_i16x8_q15mulr_sat_s
	    @simd I16x8AllTrue => visit_i16x8_all_true
	    @simd I16x8Bitmask => visit_i16x8_bitmask
	    @simd I16x8NarrowI32x4S => visit_i16x8_narrow_i32x4_s
	    @simd I16x8NarrowI32x4U => visit_i16x8_narrow_i32x4_u
	    @simd I16x8ExtendLowI8x16S => visit_i16x8_extend_low_i8x16_s
	    @simd I16x8ExtendHighI8x16S => visit_i16x8_extend_high_i8x16_s
	    @simd I16x8ExtendLowI8x16U => visit_i16x8_extend_low_i8x16_u
	    @simd I16x8ExtendHighI8x16U => visit_i16x8_extend_high_i8x16_u
	    @simd I16x8Shl => visit_i16x8_shl
	    @simd I16x8ShrS => visit_i16x8_shr_s
	    @simd I16x8ShrU => visit_i16x8_shr_u
	    @simd I16x8Add => visit_i16x8_add
	    @simd I16x8AddSatS => visit_i16x8_add_sat_s
	    @simd I16x8AddSatU => visit_i16x8_add_sat_u
	    @simd I16x8Sub => visit_i16x8_sub
	    @simd I16x8SubSatS => visit_i16x8_sub_sat_s
	    @simd I16x8SubSatU => visit_i16x8_sub_sat_u
	    @simd I16x8Mul => visit_i16x8_mul
	    @simd I16x8MinS => visit_i16x8_min_s
	    @simd I16x8MinU => visit_i16x8_min_u
	    @simd I16x8MaxS => visit_i16x8_max_s
	    @simd I16x8MaxU => visit_i16x8_max_u
	    @simd I16x8RoundingAverageU => visit_i16x8_avgr_u
	    @simd I16x8ExtMulLowI8x16S => visit_i16x8_extmul_low_i8x16_s
	    @simd I16x8ExtMulHighI8x16S => visit_i16x8_extmul_high_i8x16_s
	    @simd I16x8ExtMulLowI8x16U => visit_i16x8_extmul_low_i8x16_u
	    @simd I16x8ExtMulHighI8x16U => visit_i16x8_extmul_high_i8x16_u
	    @simd I32x4ExtAddPairwiseI16x8S => visit_i32x4_extadd_pairwise_i16x8_s
	    @simd I32x4ExtAddPairwiseI16x8U => visit_i32x4_extadd_pairwise_i16x8_u
	    @simd I32x4Abs => visit_i32x4_abs
	    @simd I32x4Neg => visit_i32x4_neg
	    @simd I32x4AllTrue => visit_i32x4_all_true
	    @simd I32x4Bitmask => visit_i32x4_bitmask
	    @simd I32x4ExtendLowI16x8S => visit_i32x4_extend_low_i16x8_s
	    @simd I32x4ExtendHighI16x8S => visit_i32x4_extend_high_i16x8_s
	    @simd I32x4ExtendLowI16x8U => visit_i32x4_extend_low_i16x8_u
	    @simd I32x4ExtendHighI16x8U => visit_i32x4_extend_high_i16x8_u
	    @simd I32x4Shl => visit_i32x4_shl
	    @simd I32x4ShrS => visit_i32x4_shr_s
	    @simd I32x4ShrU => visit_i32x4_shr_u
	    @simd I32x4Add => visit_i32x4_add
	    @simd I32x4Sub => visit_i32x4_sub
	    @simd I32x4Mul => visit_i32x4_mul
	    @simd I32x4MinS => visit_i32x4_min_s
	    @simd I32x4MinU => visit_i32x4_min_u
	    @simd I32x4MaxS => visit_i32x4_max_s
	    @simd I32x4MaxU => visit_i32x4_max_u
	    @simd I32x4DotI16x8S => visit_i32x4_dot_i16x8_s
	    @simd I32x4ExtMulLowI16x8S => visit_i32x4_extmul_low_i16x8_s
	    @simd I32x4ExtMulHighI16x8S => visit_i32x4_extmul_high_i16x8_s
	    @simd I32x4ExtMulLowI16x8U => visit_i32x4_extmul_low_i16x8_u
	    @simd I32x4ExtMulHighI16x8U => visit_i32x4_extmul_high_i16x8_u
	    @simd I64x2Abs => visit_i64x2_abs
	    @simd I64x2Neg => visit_i64x2_neg
	    @simd I64x2AllTrue => visit_i64x2_all_true
	    @simd I64x2Bitmask => visit_i64x2_bitmask
	    @simd I64x2ExtendLowI32x4S => visit_i64x2_extend_low_i32x4_s
	    @simd I64x2ExtendHighI32x4S => visit_i64x2_extend_high_i32x4_s
	    @simd I64x2ExtendLowI32x4U => visit_i64x2_extend_low_i32x4_u
	    @simd I64x2ExtendHighI32x4U => visit_i64x2_extend_high_i32x4_u
	    @simd I64x2Shl => visit_i64x2_shl
	    @simd I64x2ShrS => visit_i64x2_shr_s
	    @simd I64x2ShrU => visit_i64x2_shr_u
	    @simd I64x2Add => visit_i64x2_add
	    @simd I64x2Sub => visit_i64x2_sub
	    @simd I64x2Mul => visit_i64x2_mul
	    @simd I64x2ExtMulLowI32x4S => visit_i64x2_extmul_low_i32x4_s
	    @simd I64x2ExtMulHighI32x4S => visit_i64x2_extmul_high_i32x4_s
	    @simd I64x2ExtMulLowI32x4U => visit_i64x2_extmul_low_i32x4_u
	    @simd I64x2ExtMulHighI32x4U => visit_i64x2_extmul_high_i32x4_u
	    @simd F32x4Ceil => visit_f32x4_ceil
	    @simd F32x4Floor => visit_f32x4_floor
	    @simd F32x4Trunc => visit_f32x4_trunc
	    @simd F32x4Nearest => visit_f32x4_nearest
	    @simd F32x4Abs => visit_f32x4_abs
	    @simd F32x4Neg => visit_f32x4_neg
	    @simd F32x4Sqrt => visit_f32x4_sqrt
	    @simd F32x4Add => visit_f32x4_add
	    @simd F32x4Sub => visit_f32x4_sub
	    @simd F32x4Mul => visit_f32x4_mul
	    @simd F32x4Div => visit_f32x4_div
	    @simd F32x4Min => visit_f32x4_min
	    @simd F32x4Max => visit_f32x4_max
	    @simd F32x4PMin => visit_f32x4_pmin
	    @simd F32x4PMax => visit_f32x4_pmax
	    @simd F64x2Ceil => visit_f64x2_ceil
	    @simd F64x2Floor => visit_f64x2_floor
	    @simd F64x2Trunc => visit_f64x2_trunc
	    @simd F64x2Nearest => visit_f64x2_nearest
	    @simd F64x2Abs => visit_f64x2_abs
	    @simd F64x2Neg => visit_f64x2_neg
	    @simd F64x2Sqrt => visit_f64x2_sqrt
	    @simd F64x2Add => visit_f64x2_add
	    @simd F64x2Sub => visit_f64x2_sub
	    @simd F64x2Mul => visit_f64x2_mul
	    @simd F64x2Div => visit_f64x2_div
	    @simd F64x2Min => visit_f64x2_min
	    @simd F64x2Max => visit_f64x2_max
	    @simd F64x2PMin => visit_f64x2_pmin
	    @simd F64x2PMax => visit_f64x2_pmax
	    @simd I32x4TruncSatF32x4S => visit_i32x4_trunc_sat_f32x4_s
	    @simd I32x4TruncSatF32x4U => visit_i32x4_trunc_sat_f32x4_u
	    @simd F32x4ConvertI32x4S => visit_f32x4_convert_i32x4_s
	    @simd F32x4ConvertI32x4U => visit_f32x4_convert_i32x4_u
	    @simd I32x4TruncSatF64x2SZero => visit_i32x4_trunc_sat_f64x2_s_zero
	    @simd I32x4TruncSatF64x2UZero => visit_i32x4_trunc_sat_f64x2_u_zero
	    @simd F64x2ConvertLowI32x4S => visit_f64x2_convert_low_i32x4_s
	    @simd F64x2ConvertLowI32x4U => visit_f64x2_convert_low_i32x4_u
	    @simd F32x4DemoteF64x2Zero => visit_f32x4_demote_f64x2_zero
	    @simd F64x2PromoteLowF32x4 => visit_f64x2_promote_low_f32x4

	    // Relaxed SIMD operators
	    @relaxed_simd I8x16RelaxedSwizzle => visit_i8x16_relaxed_swizzle
	    @relaxed_simd I32x4RelaxedTruncSatF32x4S => visit_i32x4_relaxed_trunc_sat_f32x4_s
	    @relaxed_simd I32x4RelaxedTruncSatF32x4U => visit_i32x4_relaxed_trunc_sat_f32x4_u
	    @relaxed_simd I32x4RelaxedTruncSatF64x2SZero => visit_i32x4_relaxed_trunc_sat_f64x2_s_zero
	    @relaxed_simd I32x4RelaxedTruncSatF64x2UZero => visit_i32x4_relaxed_trunc_sat_f64x2_u_zero
	    @relaxed_simd F32x4RelaxedFma => visit_f32x4_relaxed_fma
	    @relaxed_simd F32x4RelaxedFnma => visit_f32x4_relaxed_fnma
	    @relaxed_simd F64x2RelaxedFma => visit_f64x2_relaxed_fma
	    @relaxed_simd F64x2RelaxedFnma => visit_f64x2_relaxed_fnma
	    @relaxed_simd I8x16RelaxedLaneselect => visit_i8x16_relaxed_laneselect
	    @relaxed_simd I16x8RelaxedLaneselect => visit_i16x8_relaxed_laneselect
	    @relaxed_simd I32x4RelaxedLaneselect => visit_i32x4_relaxed_laneselect
	    @relaxed_simd I64x2RelaxedLaneselect => visit_i64x2_relaxed_laneselect
	    @relaxed_simd F32x4RelaxedMin => visit_f32x4_relaxed_min
	    @relaxed_simd F32x4RelaxedMax => visit_f32x4_relaxed_max
	    @relaxed_simd F64x2RelaxedMin => visit_f64x2_relaxed_min
	    @relaxed_simd F64x2RelaxedMax => visit_f64x2_relaxed_max
	    @relaxed_simd I16x8RelaxedQ15mulrS => visit_i16x8_relaxed_q15mulr_s
	    @relaxed_simd I16x8DotI8x16I7x16S => visit_i16x8_dot_i8x16_i7x16_s
	    @relaxed_simd I32x4DotI8x16I7x16AddS => visit_i32x4_dot_i8x16_i7x16_add_s
	    @relaxed_simd F32x4RelaxedDotBf16x8AddF32x4 => visit_f32x4_relaxed_dot_bf16x8_add_f32x4
	}
    };
}

/// A macro to define the corresponding methods for both supported and unsupported operators
macro_rules! define {
    ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),*})? => ($visit:ident $emit:ident))*) => {
        $(
            fn $visit(&mut self, offset: usize $($(,$arg: $argty)*)?) -> Self::Output {
		self.emitop(
		    |validator| validator.$visit(offset $($(,$arg)*)?),
		    |env| env.$emit($($($arg),*)?)
		)
            }
        )*
    };

    ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),*})? => $visit:ident )*) => {
        $(
            fn $visit(&mut self, _offset: usize $($(,$arg: $argty)*)?) -> Self::Output {
		todo!(stringify!($op))
            }
        )*
    };
}

impl<'c, 'a: 'c, A, M> CodeGen<'a, 'c, A, M>
where
    A: ABI,
    M: MacroAssembler,
{
    fn emitop<V, E>(&mut self, validate: V, emitter: E) -> Result<()>
    where
        V: FnOnce(&mut FuncValidator<ValidatorResources>) -> ValidationResult<()>,
        E: FnOnce(&mut Self) -> Result<()>,
    {
        validate(&mut self.validator)
            .map_err(|e| e.into())
            .and(emitter(self))
    }

    fn emit_i32_add(&mut self) -> Result<()> {
        let is_const = self
            .context
            .stack
            .peek()
            .expect("value at stack top")
            .is_i32_const();

        if is_const {
            self.add_imm_i32();
        } else {
            self.add_i32();
        }

        Ok(())
    }

    fn add_imm_i32(&mut self) {
        let val = self
            .context
            .stack
            .pop_i32_const()
            .expect("i32 constant at stack top");
        let reg = self
            .regalloc
            .pop_to_reg(&mut self.context, OperandSize::S32);
        self.context
            .masm
            .add(RegImm::imm(val), RegImm::reg(reg), OperandSize::S32);
        self.context.stack.push(Val::reg(reg));
    }

    fn add_i32(&mut self) {
        let src = self
            .regalloc
            .pop_to_reg(&mut self.context, OperandSize::S32);
        let dst = self
            .regalloc
            .pop_to_reg(&mut self.context, OperandSize::S32);

        self.context
            .masm
            .add(RegImm::reg(src), RegImm::reg(dst), OperandSize::S32);
        self.context.stack.push(Val::reg(dst));
    }

    fn emit_i32_const(&mut self, val: i32) -> Result<()> {
        self.context.stack.push(Val::i32(val));
        Ok(())
    }

    fn emit_local_get(&mut self, index: u32) -> Result<()> {
        let context = &mut self.context;
        let slot = context
            .frame
            .get_local(index)
            .expect(&format!("valid local at slot = {}", index));
        match slot.ty {
            WasmType::I32 | WasmType::I64 => context.stack.push(Val::local(index)),
            _ => panic!("Unsupported type {} for local", slot.ty),
        }

        Ok(())
    }

    // TODO verify the case where the target local is on the stack
    fn emit_local_set(&mut self, index: u32) -> Result<()> {
        let context = &mut self.context;
        let frame = context.frame;
        let slot = frame
            .get_local(index)
            .expect(&format!("vald local at slot = {}", index));
        let size: OperandSize = slot.ty.into();
        let src = self.regalloc.pop_to_reg(context, size);
        let addr = context.masm.local_address(&slot);
        context.masm.store(RegImm::reg(src), addr, size);
        self.regalloc.free_gpr(src);

        Ok(())
    }
}

impl<'c, 'a: 'c, A, M> VisitOperator<'a> for CodeGen<'a, 'c, A, M>
where
    A: ABI,
    M: MacroAssembler,
{
    type Output = Result<()>;
    for_each_supported_op!(define);
    for_each_unsupported_op!(define);

    fn visit_end(&mut self, offset: usize) -> Result<()> {
        self.validator.visit_end(offset).map_err(|e| e.into())
    }
}

impl From<WasmType> for OperandSize {
    fn from(ty: WasmType) -> OperandSize {
        match ty {
            WasmType::I32 => OperandSize::S32,
            WasmType::I64 => OperandSize::S64,
            ty => todo!("unsupported type {}", ty),
        }
    }
}
