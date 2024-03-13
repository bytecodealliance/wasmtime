use cranelift_codegen::ir::{self, condcodes::IntCC, InstBuilder};
use cranelift_frontend::FunctionBuilder;

/// An implementation of a WebAssembly table.
#[derive(Clone)]
pub struct TableData {
    /// Global value giving the address of the start of the table.
    pub base_gv: ir::GlobalValue,

    /// Global value giving the current bound of the table, in elements.
    pub bound_gv: ir::GlobalValue,

    /// The size of a table element, in bytes.
    pub element_size: u32,
}

impl TableData {
    /// Return a CLIF value containing a native pointer to the beginning of the
    /// given index within this table.
    pub fn prepare_table_addr(
        &self,
        pos: &mut FunctionBuilder,
        mut index: ir::Value,
        addr_ty: ir::Type,
        enable_table_access_spectre_mitigation: bool,
    ) -> ir::Value {
        let index_ty = pos.func.dfg.value_type(index);

        // Start with the bounds check. Trap if `index + 1 > bound`.
        let bound = pos.ins().global_value(index_ty, self.bound_gv);

        // `index > bound - 1` is the same as `index >= bound`.
        let oob = pos
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, index, bound);
        pos.ins().trapnz(oob, ir::TrapCode::TableOutOfBounds);

        // If Spectre mitigations are enabled, we will use a comparison to
        // short-circuit the computed table element address to the start
        // of the table on the misspeculation path when out-of-bounds.
        let spectre_oob_cmp = if enable_table_access_spectre_mitigation {
            Some((index, bound))
        } else {
            None
        };

        // Convert `index` to `addr_ty`.
        if index_ty != addr_ty {
            index = pos.ins().uextend(addr_ty, index);
        }

        // Add the table base address base
        let base = pos.ins().global_value(addr_ty, self.base_gv);

        let element_size = self.element_size;
        let offset = if element_size == 1 {
            index
        } else if element_size.is_power_of_two() {
            pos.ins()
                .ishl_imm(index, i64::from(element_size.trailing_zeros()))
        } else {
            pos.ins().imul_imm(index, element_size as i64)
        };

        let element_addr = pos.ins().iadd(base, offset);

        if let Some((index, bound)) = spectre_oob_cmp {
            let cond = pos
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, index, bound);
            // If out-of-bounds, choose the table base on the misspeculation path.
            pos.ins().select_spectre_guard(cond, base, element_addr)
        } else {
            element_addr
        }
    }
}
