use crate::func_environ::FuncEnvironment;
use crate::translate::VmctxLoadChain;
use crate::trap::TranslateTrap;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::{self, InstBuilder, condcodes::IntCC, immediates::Imm64};
use cranelift_frontend::FunctionBuilder;
use wasmtime_environ::TableIndex;

/// Size of a WebAssembly table, in elements.
#[derive(Clone)]
pub enum TableSize {
    /// Non-resizable table.
    Static {
        /// Non-resizable tables have a constant size known at compile time.
        bound: u64,
    },
    /// Resizable table.
    Dynamic {
        /// Resizable tables load their current size from the `vmctx`.
        bound: VmctxLoadChain,
    },
}

impl TableSize {
    /// Get a CLIF value representing the current bounds of this table.
    pub fn bound(&self, mut pos: FuncCursor, index_ty: ir::Type) -> ir::Value {
        match self {
            // Instead of `i64::try_from(bound)`, here we just want to directly interpret `bound` as an i64.
            TableSize::Static { bound } => pos.ins().iconst(index_ty, Imm64::new(*bound as i64)),
            TableSize::Dynamic { bound } => {
                let vmctx = vmctx(&mut pos);
                let bound = bound.emit(&mut pos, vmctx);
                let ty = pos.func.dfg.value_type(bound);
                if index_ty == ty {
                    bound
                } else if index_ty.bytes() < ty.bytes() {
                    pos.ins().ireduce(index_ty, bound)
                } else {
                    pos.ins().uextend(index_ty, bound)
                }
            }
        }
    }
}

/// An implementation of a WebAssembly table.
#[derive(Clone)]
pub struct TableData {
    /// The load chain giving the address of the start of the table.
    pub base: VmctxLoadChain,

    /// The size of the table, in elements.
    pub bound: TableSize,

    /// The size of a table element, in bytes.
    pub element_size: u32,
}

impl TableData {
    /// Return a CLIF value containing a native pointer to the beginning of the
    /// given index within this table.
    pub fn prepare_table_addr(
        &self,
        env: &mut FuncEnvironment<'_>,
        pos: &mut FunctionBuilder,
        mut index: ir::Value,
        table_index: TableIndex,
    ) -> (ir::Value, ir::MemFlagsData) {
        let index_ty = pos.func.dfg.value_type(index);
        let addr_ty = env.pointer_type();
        let spectre_mitigations_enabled =
            env.isa().flags().enable_table_access_spectre_mitigation()
                && env.clif_memory_traps_enabled();

        // Start with the bounds check. Trap if `index + 1 > bound`.
        let bound = self.bound.bound(pos.cursor(), index_ty);

        // `index > bound - 1` is the same as `index >= bound`.
        let oob = pos
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, index, bound);

        if !spectre_mitigations_enabled {
            env.trapnz(pos, oob, crate::TRAP_TABLE_OUT_OF_BOUNDS);
        }

        // Convert `index` to `addr_ty`.
        if addr_ty.bytes() > index_ty.bytes() {
            index = pos.ins().uextend(addr_ty, index);
        } else if addr_ty.bytes() < index_ty.bytes() {
            index = pos.ins().ireduce(addr_ty, index);
        }

        // Add the table base address base
        let vmctx = vmctx(&mut pos.cursor());
        let base = self.base.emit(&mut pos.cursor(), vmctx);

        let element_size = self.element_size;
        let offset = if element_size == 1 {
            index
        } else if element_size.is_power_of_two() {
            pos.ins()
                .ishl_imm_u(index, i64::from(element_size.trailing_zeros()))
        } else {
            pos.ins().imul_imm_s(index, element_size as i64)
        };

        let element_addr = pos.ins().iadd(base, offset);

        let region = env.table_alias_region(pos.func, table_index);
        let base_flags = ir::MemFlagsData::new()
            .with_aligned()
            .with_alias_region(Some(region));
        if spectre_mitigations_enabled {
            // Short-circuit the computed table element address to a null pointer
            // when out-of-bounds. The consumer of this address will trap when
            // trying to access it.
            let zero = pos.ins().iconst(addr_ty, 0);
            (
                pos.ins().select_spectre_guard(oob, zero, element_addr),
                base_flags.with_trap_code(Some(crate::TRAP_TABLE_OUT_OF_BOUNDS)),
            )
        } else {
            (element_addr, base_flags.with_trap_code(None))
        }
    }
}

/// Read the `vmctx` value out of the function's special `VMContext` parameter.
fn vmctx(pos: &mut FuncCursor<'_>) -> ir::Value {
    pos.func
        .special_param(ir::ArgumentPurpose::VMContext)
        .expect("missing vmctx parameter")
}
