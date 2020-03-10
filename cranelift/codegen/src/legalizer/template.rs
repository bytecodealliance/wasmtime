//! Expansion of templates.
//!
//! This module exports the `expand_template` function which transforms a `template`
//! instruction into code that depends on the kind of template referenced.

use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::{self, InstBuilder};
use crate::isa::TargetIsa;

/// Expand a `template` instruction.
pub fn expand_template(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    // Unpack the instruction.
    let template = match func.dfg[inst] {
        ir::InstructionData::UnaryTemplate { opcode, template } => {
            debug_assert_eq!(opcode, ir::Opcode::Template);
            template
        }
        _ => panic!("Wanted template: {}", func.dfg.display_inst(inst, None)),
    };

    match func.templates[template] {
        ir::TemplateData::VMContext => vmctx_addr(inst, func),
        ir::TemplateData::IAddImm { base, offset } => {
            iadd_imm_addr(inst, func, base, offset.into(), isa)
        }
        ir::TemplateData::Load {
            base,
            offset,
            result_type,
            readonly,
        } => load_addr(inst, func, base, offset, result_type, readonly, isa),
        ir::TemplateData::Symbol { tls, .. } => symbol(inst, func, template, isa, tls),
        ir::TemplateData::Call { .. } => unimplemented!("call template expansion"),
        ir::TemplateData::IfElse { .. } => unimplemented!("if-else template expansion"),
    }
}

/// Expand a `template` instruction for vmctx.
fn vmctx_addr(inst: ir::Inst, func: &mut ir::Function) {
    // Get the value representing the `vmctx` argument.
    let vmctx = func
        .special_param(ir::ArgumentPurpose::VMContext)
        .expect("Missing vmctx parameter");

    // Replace the `template` instruction's value with an alias to the vmctx arg.
    let result = func.dfg.first_result(inst);
    func.dfg.clear_results(inst);
    func.dfg.change_to_alias(result, vmctx);
    func.layout.remove_inst(inst);
}

/// Expand a `template` instruction for an iadd_imm.
fn iadd_imm_addr(
    inst: ir::Inst,
    func: &mut ir::Function,
    base: ir::Template,
    offset: i64,
    isa: &dyn TargetIsa,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);

    // Get the value for the lhs. For tidiness, expand VMContext here so that we avoid
    // `vmctx_addr` which creates an otherwise unneeded value alias.
    let lhs = if let ir::TemplateData::VMContext = pos.func.templates[base] {
        pos.func
            .special_param(ir::ArgumentPurpose::VMContext)
            .expect("Missing vmctx parameter")
    } else {
        let result_type = pos.func.template_result_type(base, isa);
        pos.ins().template(result_type, base)
    };

    // Simply replace the `template` instruction with an `iadd_imm`, reusing the result value.
    pos.func.dfg.replace(inst).iadd_imm(lhs, offset);
}

/// Expand a `template` instruction for a load.
fn load_addr(
    inst: ir::Inst,
    func: &mut ir::Function,
    base: ir::Template,
    offset: ir::immediates::Offset32,
    result_type: ir::Type,
    readonly: bool,
    isa: &dyn TargetIsa,
) {
    // We need to load a pointer from the `base`, so insert a new `template`
    // instruction. This depends on the iterative legalization loop. Note that the IR verifier
    // detects any cycles in the `load` templates.
    let ptr_ty = isa.pointer_type();
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Get the value for the base. For tidiness, expand VMContext here so that we avoid
    // `vmctx_addr` which creates an otherwise unneeded value alias.
    let base_addr = if let ir::TemplateData::VMContext = pos.func.templates[base] {
        pos.func
            .special_param(ir::ArgumentPurpose::VMContext)
            .expect("Missing vmctx parameter")
    } else {
        pos.ins().template(ptr_ty, base)
    };

    // Template loads are always notrap and aligned. They may be readonly.
    let mut mflags = ir::MemFlags::trusted();
    if readonly {
        mflags.set_readonly();
    }

    // Perform the load.
    pos.func
        .dfg
        .replace(inst)
        .load(result_type, mflags, base_addr, offset);
}

/// Expand a `template` instruction for a symbolic name global.
fn symbol(
    inst: ir::Inst,
    func: &mut ir::Function,
    template: ir::Template,
    isa: &dyn TargetIsa,
    tls: bool,
) {
    let ptr_ty = isa.pointer_type();

    if tls {
        func.dfg.replace(inst).tls_value(ptr_ty, template);
    } else {
        func.dfg.replace(inst).symbol_value(ptr_ty, template);
    }
}
