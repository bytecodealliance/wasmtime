use crate::backend::{
    ret_locs, BlockCallingConvention, CodeGenSession, Context, Label, Registers, ValueLocation,
    VirtualCallingConvention,
};
use crate::error::Error;
use crate::microwasm::*;
use crate::module::{ModuleContext, SigType, Signature};
use cranelift_codegen::binemit;
use dynasmrt::DynasmApi;
use either::{Either, Left, Right};
use more_asserts::assert_ge;
use multi_mut::HashMapMultiMut;
use std::{collections::HashMap, hash::Hash};
use std::{fmt, mem};

#[derive(Debug)]
struct Block {
    label: BrTarget<Label>,
    calling_convention: Option<Either<BlockCallingConvention, VirtualCallingConvention>>,
    params: u32,
    // TODO: Is there a cleaner way to do this? `has_backwards_callers` should always be set if `is_next`
    //       is false, so we should probably use an `enum` here.
    is_next: bool,
    num_callers: Option<u32>,
    actual_num_callers: u32,
    has_backwards_callers: bool,
}

impl Block {
    fn should_serialize_args(&self) -> bool {
        self.calling_convention.is_none()
            && (self.num_callers != Some(1) || self.has_backwards_callers)
    }
}

const DISASSEMBLE: bool = false;

pub fn translate_wasm<M>(
    session: &mut CodeGenSession<M>,
    reloc_sink: &mut dyn binemit::RelocSink,
    func_idx: u32,
    body: &wasmparser::FunctionBody,
) -> Result<(), Error>
where
    M: ModuleContext,
    for<'any> &'any M::Signature: Into<OpSig>,
{
    let ty = session.module_context.defined_func_type(func_idx);

    if DISASSEMBLE {
        let microwasm_conv = MicrowasmConv::new(
            session.module_context,
            ty.params().iter().map(SigType::to_microwasm_type),
            ty.returns().iter().map(SigType::to_microwasm_type),
            body,
        )?;

        let _ = crate::microwasm::dis(
            std::io::stdout(),
            func_idx,
            microwasm_conv.flat_map(|ops| ops.unwrap()),
        );
    }

    let microwasm_conv = MicrowasmConv::new(
        session.module_context,
        ty.params().iter().map(SigType::to_microwasm_type),
        ty.returns().iter().map(SigType::to_microwasm_type),
        body,
    )?;

    let mut body = Vec::new();
    for i in microwasm_conv {
        match i {
            Ok(v) => body.extend(v),
            Err(e) => return Err(Error::Microwasm(e.message.to_string())),
        };
    }

    translate(session, reloc_sink, func_idx, body)?;
    Ok(())
}

pub fn translate<M, I, L: Send + Sync + 'static>(
    session: &mut CodeGenSession<M>,
    reloc_sink: &mut dyn binemit::RelocSink,
    func_idx: u32,
    body: I,
) -> Result<(), Error>
where
    M: ModuleContext,
    I: IntoIterator<Item = Operator<L>>,
    L: Hash + Clone + Eq,
    BrTarget<L>: std::fmt::Display,
{
    fn drop_elements<T>(stack: &mut Vec<T>, depths: std::ops::RangeInclusive<u32>) {
        let _ = (|| {
            let start = stack
                .len()
                .checked_sub(1)?
                .checked_sub(*depths.end() as usize)?;
            let end = stack
                .len()
                .checked_sub(1)?
                .checked_sub(*depths.start() as usize)?;
            let real_range = start..=end;

            stack.drain(real_range);

            Some(())
        })();
    }

    let func_type = session.module_context.defined_func_type(func_idx);
    let mut body = body.into_iter().peekable();

    let module_context = &*session.module_context;
    let mut op_offset_map = mem::replace(&mut session.op_offset_map, vec![]);
    let ctx = &mut session.new_context(func_idx, reloc_sink);
    op_offset_map.push((
        ctx.asm.offset(),
        Box::new(format!("Function {}:", func_idx)),
    ));

    let params = func_type
        .params()
        .iter()
        .map(|t| t.to_microwasm_type())
        .collect::<Vec<_>>();

    ctx.start_function(params.iter().cloned());

    let mut blocks = HashMap::<BrTarget<L>, Block>::new();

    let num_returns = func_type.returns().len();

    blocks.insert(
        BrTarget::Return,
        Block {
            label: BrTarget::Return,
            params: num_returns as u32,
            calling_convention: Some(Left(BlockCallingConvention::function_start(ret_locs(
                func_type.returns().iter().map(|t| t.to_microwasm_type()),
            )))),
            is_next: false,
            has_backwards_callers: false,
            actual_num_callers: 0,
            num_callers: None,
        },
    );

    while let Some(op) = body.next() {
        if let Some(Operator::Label(label)) = body.peek() {
            let block = blocks
                .get_mut(&BrTarget::Label(label.clone()))
                .expect("Label defined before being declared");
            block.is_next = true;
        }

        // `cfg` on blocks doesn't work in the compiler right now, so we have to write a dummy macro
        #[cfg(debug_assertions)]
        macro_rules! assertions {
            () => {
                if let Operator::Label(label) = &op {
                    let block = &blocks[&BrTarget::Label(label.clone())];
                    let num_cc_params = block.calling_convention.as_ref().map(|cc| match cc {
                        Left(cc) => cc.arguments.len(),
                        Right(cc) => cc.stack.len(),
                    });
                    if let Some(num_cc_params) = num_cc_params {
                        assert_ge!(num_cc_params, block.params as usize);
                    }
                } else {
                    let mut actual_regs = Registers::new();
                    for val in &ctx.block_state.stack {
                        if let ValueLocation::Reg(gpr) = val {
                            actual_regs.mark_used(*gpr);
                        }
                    }
                    assert_eq!(actual_regs, ctx.block_state.regs);
                }
            };
        }

        #[cfg(not(debug_assertions))]
        macro_rules! assertions {
            () => {};
        }

        assertions!();

        struct DisassemblyOpFormatter<Label>(Operator<Label>);

        impl<Label> fmt::Display for DisassemblyOpFormatter<Label>
        where
            Operator<Label>: fmt::Display,
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self.0 {
                    Operator::Label(_) => write!(f, "{}", self.0),
                    Operator::Block { .. } => write!(f, "{:5}\t{}", "", self.0),
                    _ => write!(f, "{:5}\t  {}", "", self.0),
                }
            }
        }

        op_offset_map.push((
            ctx.asm.offset(),
            Box::new(DisassemblyOpFormatter(op.clone())),
        ));

        match op {
            Operator::Unreachable => {
                ctx.trap();
            }
            Operator::Label(label) => {
                use std::collections::hash_map::Entry;

                if let Entry::Occupied(mut entry) = blocks.entry(BrTarget::Label(label.clone())) {
                    let has_backwards_callers = {
                        let block = entry.get_mut();

                        // TODO: Maybe we want to restrict Microwasm so that at least one of its callers
                        //       must be before the label. In an ideal world the restriction would be that
                        //       blocks without callers are illegal, but that's not reasonably possible for
                        //       Microwasm generated from Wasm.
                        if block.actual_num_callers == 0 {
                            loop {
                                let done = match body.peek() {
                                    Some(Operator::Label(_)) | None => true,
                                    Some(_) => false,
                                };

                                if done {
                                    break;
                                }

                                let skipped = body.next();

                                // We still want to honour block definitions even in unreachable code
                                if let Some(Operator::Block {
                                    label,
                                    has_backwards_callers,
                                    params,
                                    num_callers,
                                }) = skipped
                                {
                                    let asm_label = ctx.create_label();
                                    blocks.insert(
                                        BrTarget::Label(label),
                                        Block {
                                            label: BrTarget::Label(asm_label),
                                            params: params.len() as _,
                                            calling_convention: None,
                                            is_next: false,
                                            has_backwards_callers,
                                            actual_num_callers: 0,
                                            num_callers,
                                        },
                                    );
                                }
                            }

                            continue;
                        }

                        block.is_next = false;

                        // TODO: We can `take` this if it's a `Right`
                        match block.calling_convention.as_ref() {
                            Some(Left(cc)) => {
                                ctx.apply_cc(cc);
                            }
                            Some(Right(virt)) => {
                                ctx.set_state(virt.clone());
                            }
                            _ => assert_eq!(block.params as usize, ctx.block_state.stack.len()),
                        }

                        ctx.define_label(block.label.label().unwrap().clone());

                        block.has_backwards_callers
                    };

                    // To reduce memory overhead
                    if !has_backwards_callers {
                        entry.remove_entry();
                    }
                } else {
                    return Err(Error::Microwasm(
                        "Label defined before being declared".to_string(),
                    ));
                }
            }
            Operator::Block {
                label,
                has_backwards_callers,
                params,
                num_callers,
            } => {
                let asm_label = ctx.create_label();
                blocks.insert(
                    BrTarget::Label(label),
                    Block {
                        label: BrTarget::Label(asm_label),
                        params: params.len() as _,
                        calling_convention: None,
                        is_next: false,
                        has_backwards_callers,
                        actual_num_callers: 0,
                        num_callers,
                    },
                );
            }
            Operator::Br { target } => {
                // TODO: We should add the block to the hashmap if we don't have it already
                let block = blocks.get_mut(&target).unwrap();
                block.actual_num_callers += 1;

                let should_serialize_args = block.should_serialize_args();

                match block {
                    Block {
                        is_next,
                        label: BrTarget::Label(l),
                        calling_convention,
                        ..
                    } => {
                        let cc = if should_serialize_args {
                            *calling_convention = Some(Left(ctx.serialize_args(block.params)));
                            None
                        } else {
                            calling_convention
                                .as_ref()
                                .map(Either::as_ref)
                                .and_then(Either::left)
                        };

                        if let Some(cc) = cc {
                            ctx.pass_block_args(cc);
                        }

                        if !*is_next {
                            ctx.br(*l);
                        }
                    }
                    Block {
                        label: BrTarget::Return,
                        calling_convention: Some(Left(cc)),
                        ..
                    } => {
                        ctx.pass_block_args(cc);
                        ctx.ret();
                    }
                    _ => unimplemented!(),
                }
            }
            Operator::BrIf { then, else_ } => {
                let (then_block, else_block) = blocks.pair_mut(&then.target, &else_.target);
                // TODO: If actual_num_callers == num_callers then we can remove this block from the hashmap.
                //       This frees memory and acts as a kind of verification that `num_callers` is set
                //       correctly. It doesn't help for loops and block ends generated from Wasm.
                then_block.actual_num_callers += 1;
                else_block.actual_num_callers += 1;

                let then_block_parts = (then_block.is_next, then_block.label);
                let else_block_parts = (else_block.is_next, else_block.label);

                // TODO: The blocks should have compatible (one must be subset of other?) calling
                //       conventions or else at least one must have no calling convention. This
                //       should always be true for converting from WebAssembly AIUI.
                let f = |ctx: &mut Context<_>| {
                    let then_block_should_serialize_args = then_block.should_serialize_args();
                    let else_block_should_serialize_args = else_block.should_serialize_args();
                    let max_params = then_block.params.max(else_block.params);

                    match (
                        (&mut then_block.calling_convention, &then.to_drop),
                        (&mut else_block.calling_convention, &else_.to_drop),
                    ) {
                        ((Some(Left(ref cc)), _), ref mut other @ (None, _))
                        | (ref mut other @ (None, _), (Some(Left(ref cc)), _)) => {
                            let mut cc = ctx.serialize_block_args(cc, max_params);
                            if let Some(to_drop) = other.1 {
                                drop_elements(&mut cc.arguments, to_drop.clone());
                            }
                            *other.0 = Some(Left(cc));
                        }
                        (
                            (ref mut then_cc @ None, then_to_drop),
                            (ref mut else_cc @ None, else_to_drop),
                        ) => {
                            let virt_cc = if !then_block_should_serialize_args
                                || !else_block_should_serialize_args
                            {
                                Some(ctx.virtual_calling_convention())
                            } else {
                                None
                            };
                            let cc = if then_block_should_serialize_args
                                || else_block_should_serialize_args
                            {
                                Some(ctx.serialize_args(max_params))
                            } else {
                                None
                            };

                            **then_cc = if then_block_should_serialize_args {
                                let mut cc = cc.clone().unwrap();
                                if let Some(to_drop) = then_to_drop.clone() {
                                    drop_elements(&mut cc.arguments, to_drop);
                                }
                                Some(Left(cc))
                            } else {
                                let mut cc = virt_cc.clone().unwrap();
                                if let Some(to_drop) = then_to_drop.clone() {
                                    drop_elements(&mut cc.stack, to_drop);
                                }
                                Some(Right(cc))
                            };
                            **else_cc = if else_block_should_serialize_args {
                                let mut cc = cc.unwrap();
                                if let Some(to_drop) = else_to_drop.clone() {
                                    drop_elements(&mut cc.arguments, to_drop);
                                }
                                Some(Left(cc))
                            } else {
                                let mut cc = virt_cc.unwrap();
                                if let Some(to_drop) = else_to_drop.clone() {
                                    drop_elements(&mut cc.stack, to_drop);
                                }
                                Some(Right(cc))
                            };
                        }
                        _ => unimplemented!(
                            "Can't pass different params to different sides of `br_if` yet"
                        ),
                    }
                };

                match (then_block_parts, else_block_parts) {
                    ((true, _), (false, else_)) => {
                        ctx.br_if_false(else_, f);
                    }
                    ((false, then), (true, _)) => {
                        ctx.br_if_true(then, f);
                    }
                    ((false, then), (false, else_)) => {
                        ctx.br_if_true(then, f);
                        ctx.br(else_);
                    }
                    other => unimplemented!("{:#?}", other),
                }
            }
            Operator::BrTable(BrTable { targets, default }) => {
                use itertools::Itertools;

                let (label, num_callers, params) = {
                    let def = &blocks[&default.target];
                    (
                        if def.is_next { None } else { Some(def.label) },
                        def.num_callers,
                        def.params
                            + default
                                .to_drop
                                .as_ref()
                                .map(|t| t.clone().count())
                                .unwrap_or_default() as u32,
                    )
                };

                let target_labels = targets
                    .iter()
                    .map(|target| {
                        let block = &blocks[&target.target];
                        if block.is_next {
                            None
                        } else {
                            Some(block.label)
                        }
                    })
                    .collect::<Vec<_>>();

                ctx.br_table(target_labels, label, |ctx| {
                    let mut cc = None;
                    let mut max_params = params;
                    let mut max_num_callers = num_callers;

                    for target in targets.iter().chain(std::iter::once(&default)).unique() {
                        let block = blocks.get_mut(&target.target).unwrap();
                        block.actual_num_callers += 1;

                        if block.calling_convention.is_some() {
                            let new_cc = block.calling_convention.clone();
                            assert!(
                                cc.is_none() || cc == new_cc,
                                "Can't pass different params to different elements of `br_table` \
                                 yet"
                            );
                            cc = new_cc;
                        }

                        if let Some(max) = max_num_callers {
                            max_num_callers = block.num_callers.map(|n| max.max(n));
                        }

                        max_params = max_params.max(
                            block.params
                                + target
                                    .to_drop
                                    .as_ref()
                                    .map(|t| t.clone().count())
                                    .unwrap_or_default() as u32,
                        );
                    }

                    let cc = cc
                        .map(|cc| match cc {
                            Left(cc) => Left(ctx.serialize_block_args(&cc, max_params)),
                            Right(cc) => Right(cc),
                        })
                        .unwrap_or_else(|| {
                            if max_num_callers.map(|callers| callers <= 1).unwrap_or(false) {
                                Right(ctx.virtual_calling_convention())
                            } else {
                                Left(ctx.serialize_args(max_params))
                            }
                        });

                    for target in targets.iter().chain(std::iter::once(&default)).unique() {
                        let block = blocks.get_mut(&target.target).unwrap();
                        let mut cc = cc.clone();
                        if let Some(to_drop) = target.to_drop.clone() {
                            match &mut cc {
                                Left(cc) => drop_elements(&mut cc.arguments, to_drop),
                                Right(cc) => drop_elements(&mut cc.stack, to_drop),
                            }
                        }
                        block.calling_convention = Some(cc);
                    }
                });
            }
            Operator::Swap(depth) => ctx.swap(depth),
            Operator::Pick(depth) => ctx.pick(depth),
            Operator::Eq(I32) => ctx.i32_eq(),
            Operator::Eqz(Size::_32) => ctx.i32_eqz(),
            Operator::Ne(I32) => ctx.i32_neq(),
            Operator::Lt(SI32) => ctx.i32_lt_s(),
            Operator::Le(SI32) => ctx.i32_le_s(),
            Operator::Gt(SI32) => ctx.i32_gt_s(),
            Operator::Ge(SI32) => ctx.i32_ge_s(),
            Operator::Lt(SU32) => ctx.i32_lt_u(),
            Operator::Le(SU32) => ctx.i32_le_u(),
            Operator::Gt(SU32) => ctx.i32_gt_u(),
            Operator::Ge(SU32) => ctx.i32_ge_u(),
            Operator::Add(I32) => ctx.i32_add(),
            Operator::Sub(I32) => ctx.i32_sub(),
            Operator::And(Size::_32) => ctx.i32_and(),
            Operator::Or(Size::_32) => ctx.i32_or(),
            Operator::Xor(Size::_32) => ctx.i32_xor(),
            Operator::Mul(I32) => ctx.i32_mul(),
            Operator::Div(SU32) => ctx.i32_div_u(),
            Operator::Div(SI32) => ctx.i32_div_s(),
            Operator::Rem(sint::I32) => ctx.i32_rem_s(),
            Operator::Rem(sint::U32) => ctx.i32_rem_u(),
            Operator::Shl(Size::_32) => ctx.i32_shl(),
            Operator::Shr(sint::I32) => ctx.i32_shr_s(),
            Operator::Shr(sint::U32) => ctx.i32_shr_u(),
            Operator::Rotl(Size::_32) => ctx.i32_rotl(),
            Operator::Rotr(Size::_32) => ctx.i32_rotr(),
            Operator::Clz(Size::_32) => ctx.i32_clz(),
            Operator::Ctz(Size::_32) => ctx.i32_ctz(),
            Operator::Popcnt(Size::_32) => ctx.i32_popcnt(),
            Operator::Eq(I64) => ctx.i64_eq(),
            Operator::Eqz(Size::_64) => ctx.i64_eqz(),
            Operator::Ne(I64) => ctx.i64_neq(),
            Operator::Lt(SI64) => ctx.i64_lt_s(),
            Operator::Le(SI64) => ctx.i64_le_s(),
            Operator::Gt(SI64) => ctx.i64_gt_s(),
            Operator::Ge(SI64) => ctx.i64_ge_s(),
            Operator::Lt(SU64) => ctx.i64_lt_u(),
            Operator::Le(SU64) => ctx.i64_le_u(),
            Operator::Gt(SU64) => ctx.i64_gt_u(),
            Operator::Ge(SU64) => ctx.i64_ge_u(),
            Operator::Add(I64) => ctx.i64_add(),
            Operator::Sub(I64) => ctx.i64_sub(),
            Operator::And(Size::_64) => ctx.i64_and(),
            Operator::Or(Size::_64) => ctx.i64_or(),
            Operator::Xor(Size::_64) => ctx.i64_xor(),
            Operator::Mul(I64) => ctx.i64_mul(),
            Operator::Div(SU64) => ctx.i64_div_u(),
            Operator::Div(SI64) => ctx.i64_div_s(),
            Operator::Rem(sint::I64) => ctx.i64_rem_s(),
            Operator::Rem(sint::U64) => ctx.i64_rem_u(),
            Operator::Shl(Size::_64) => ctx.i64_shl(),
            Operator::Shr(sint::I64) => ctx.i64_shr_s(),
            Operator::Shr(sint::U64) => ctx.i64_shr_u(),
            Operator::Rotl(Size::_64) => ctx.i64_rotl(),
            Operator::Rotr(Size::_64) => ctx.i64_rotr(),
            Operator::Clz(Size::_64) => ctx.i64_clz(),
            Operator::Ctz(Size::_64) => ctx.i64_ctz(),
            Operator::Popcnt(Size::_64) => ctx.i64_popcnt(),
            Operator::Add(F32) => ctx.f32_add(),
            Operator::Mul(F32) => ctx.f32_mul(),
            Operator::Sub(F32) => ctx.f32_sub(),
            Operator::Div(SF32) => ctx.f32_div(),
            Operator::Min(Size::_32) => ctx.f32_min(),
            Operator::Max(Size::_32) => ctx.f32_max(),
            Operator::Copysign(Size::_32) => ctx.f32_copysign(),
            Operator::Sqrt(Size::_32) => ctx.f32_sqrt(),
            Operator::Neg(Size::_32) => ctx.f32_neg(),
            Operator::Abs(Size::_32) => ctx.f32_abs(),
            Operator::Floor(Size::_32) => ctx.f32_floor(),
            Operator::Ceil(Size::_32) => ctx.f32_ceil(),
            Operator::Nearest(Size::_32) => ctx.f32_nearest(),
            Operator::Trunc(Size::_32) => ctx.f32_trunc(),
            Operator::Eq(F32) => ctx.f32_eq(),
            Operator::Ne(F32) => ctx.f32_ne(),
            Operator::Gt(SF32) => ctx.f32_gt(),
            Operator::Ge(SF32) => ctx.f32_ge(),
            Operator::Lt(SF32) => ctx.f32_lt(),
            Operator::Le(SF32) => ctx.f32_le(),
            Operator::Add(F64) => ctx.f64_add(),
            Operator::Mul(F64) => ctx.f64_mul(),
            Operator::Sub(F64) => ctx.f64_sub(),
            Operator::Div(SF64) => ctx.f64_div(),
            Operator::Min(Size::_64) => ctx.f64_min(),
            Operator::Max(Size::_64) => ctx.f64_max(),
            Operator::Copysign(Size::_64) => ctx.f64_copysign(),
            Operator::Sqrt(Size::_64) => ctx.f64_sqrt(),
            Operator::Neg(Size::_64) => ctx.f64_neg(),
            Operator::Abs(Size::_64) => ctx.f64_abs(),
            Operator::Floor(Size::_64) => ctx.f64_floor(),
            Operator::Ceil(Size::_64) => ctx.f64_ceil(),
            Operator::Nearest(Size::_64) => ctx.f64_nearest(),
            Operator::Trunc(Size::_64) => ctx.f64_trunc(),
            Operator::Eq(F64) => ctx.f64_eq(),
            Operator::Ne(F64) => ctx.f64_ne(),
            Operator::Gt(SF64) => ctx.f64_gt(),
            Operator::Ge(SF64) => ctx.f64_ge(),
            Operator::Lt(SF64) => ctx.f64_lt(),
            Operator::Le(SF64) => ctx.f64_le(),
            Operator::Drop(range) => ctx.drop(range),
            Operator::Const(val) => ctx.const_(val),
            Operator::I32WrapFromI64 => ctx.i32_wrap_from_i64(),
            Operator::I32ReinterpretFromF32 => ctx.i32_reinterpret_from_f32(),
            Operator::I64ReinterpretFromF64 => ctx.i64_reinterpret_from_f64(),
            Operator::F32ReinterpretFromI32 => ctx.f32_reinterpret_from_i32(),
            Operator::F64ReinterpretFromI64 => ctx.f64_reinterpret_from_i64(),
            Operator::ITruncFromF {
                input_ty: Size::_32,
                output_ty: sint::I32,
            } => {
                ctx.i32_truncate_f32_s();
            }
            Operator::ITruncFromF {
                input_ty: Size::_32,
                output_ty: sint::U32,
            } => {
                ctx.i32_truncate_f32_u();
            }
            Operator::ITruncFromF {
                input_ty: Size::_64,
                output_ty: sint::I32,
            } => {
                ctx.i32_truncate_f64_s();
            }
            Operator::ITruncFromF {
                input_ty: Size::_64,
                output_ty: sint::U32,
            } => {
                ctx.i32_truncate_f64_u();
            }
            Operator::ITruncFromF {
                input_ty: Size::_32,
                output_ty: sint::I64,
            } => {
                ctx.i64_truncate_f32_s();
            }
            Operator::ITruncFromF {
                input_ty: Size::_32,
                output_ty: sint::U64,
            } => {
                ctx.i64_truncate_f32_u();
            }
            Operator::ITruncFromF {
                input_ty: Size::_64,
                output_ty: sint::I64,
            } => {
                ctx.i64_truncate_f64_s();
            }
            Operator::ITruncFromF {
                input_ty: Size::_64,
                output_ty: sint::U64,
            } => {
                ctx.i64_truncate_f64_u();
            }
            Operator::Extend {
                sign: Signedness::Unsigned,
            } => ctx.i32_extend_u(),
            Operator::Extend {
                sign: Signedness::Signed,
            } => ctx.i32_extend_s(),
            Operator::FConvertFromI {
                input_ty: sint::I32,
                output_ty: Size::_32,
            } => ctx.f32_convert_from_i32_s(),
            Operator::FConvertFromI {
                input_ty: sint::I32,
                output_ty: Size::_64,
            } => ctx.f64_convert_from_i32_s(),
            Operator::FConvertFromI {
                input_ty: sint::I64,
                output_ty: Size::_32,
            } => ctx.f32_convert_from_i64_s(),
            Operator::FConvertFromI {
                input_ty: sint::I64,
                output_ty: Size::_64,
            } => ctx.f64_convert_from_i64_s(),
            Operator::FConvertFromI {
                input_ty: sint::U32,
                output_ty: Size::_32,
            } => ctx.f32_convert_from_i32_u(),
            Operator::FConvertFromI {
                input_ty: sint::U32,
                output_ty: Size::_64,
            } => ctx.f64_convert_from_i32_u(),
            Operator::FConvertFromI {
                input_ty: sint::U64,
                output_ty: Size::_32,
            } => ctx.f32_convert_from_i64_u(),
            Operator::FConvertFromI {
                input_ty: sint::U64,
                output_ty: Size::_64,
            } => ctx.f64_convert_from_i64_u(),
            Operator::F64PromoteFromF32 => ctx.f64_from_f32(),
            Operator::F32DemoteFromF64 => ctx.f32_from_f64(),
            Operator::Load8 {
                ty: sint::U32,
                memarg,
            } => ctx.i32_load8_u(memarg.offset),
            Operator::Load16 {
                ty: sint::U32,
                memarg,
            } => ctx.i32_load16_u(memarg.offset),
            Operator::Load8 {
                ty: sint::I32,
                memarg,
            } => ctx.i32_load8_s(memarg.offset),
            Operator::Load16 {
                ty: sint::I32,
                memarg,
            } => ctx.i32_load16_s(memarg.offset),
            Operator::Load8 {
                ty: sint::U64,
                memarg,
            } => ctx.i64_load8_u(memarg.offset),
            Operator::Load16 {
                ty: sint::U64,
                memarg,
            } => ctx.i64_load16_u(memarg.offset),
            Operator::Load8 {
                ty: sint::I64,
                memarg,
            } => ctx.i64_load8_s(memarg.offset),
            Operator::Load16 {
                ty: sint::I64,
                memarg,
            } => ctx.i64_load16_s(memarg.offset),
            Operator::Load32 {
                sign: Signedness::Unsigned,
                memarg,
            } => ctx.i64_load32_u(memarg.offset),
            Operator::Load32 {
                sign: Signedness::Signed,
                memarg,
            } => ctx.i64_load32_s(memarg.offset),
            Operator::Load { ty: I32, memarg } => ctx.i32_load(memarg.offset),
            Operator::Load { ty: F32, memarg } => ctx.f32_load(memarg.offset),
            Operator::Load { ty: I64, memarg } => ctx.i64_load(memarg.offset),
            Operator::Load { ty: F64, memarg } => ctx.f64_load(memarg.offset),
            Operator::Store8 { ty: _, memarg } => ctx.store8(memarg.offset),
            Operator::Store16 { ty: _, memarg } => ctx.store16(memarg.offset),
            Operator::Store32 { memarg }
            | Operator::Store { ty: I32, memarg }
            | Operator::Store { ty: F32, memarg } => ctx.store32(memarg.offset),
            Operator::Store { ty: I64, memarg } | Operator::Store { ty: F64, memarg } => {
                ctx.store64(memarg.offset)
            }
            Operator::GetGlobal(idx) => ctx.get_global(idx),
            Operator::SetGlobal(idx) => ctx.set_global(idx),
            Operator::Select => {
                ctx.select();
            }
            Operator::MemorySize { reserved: _ } => {
                ctx.memory_size();
            }
            Operator::MemoryGrow { reserved: _ } => {
                ctx.memory_grow();
            }
            Operator::Call { function_index } => {
                let callee_ty = module_context.func_type(function_index);

                if let Some(defined_index) = module_context.defined_func_index(function_index) {
                    if function_index == func_idx {
                        ctx.call_direct_self(
                            defined_index,
                            callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                            callee_ty.returns().iter().map(|t| t.to_microwasm_type()),
                        );
                    } else {
                        ctx.call_direct(
                            function_index,
                            callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                            callee_ty.returns().iter().map(|t| t.to_microwasm_type()),
                        );
                    }
                } else {
                    ctx.call_direct_imported(
                        function_index,
                        callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                        callee_ty.returns().iter().map(|t| t.to_microwasm_type()),
                    );
                }
            }
            Operator::CallIndirect {
                type_index,
                table_index,
            } => {
                assert_eq!(table_index, 0);

                let callee_ty = module_context.signature(type_index);

                // TODO: this implementation assumes that this function is locally defined.

                ctx.call_indirect(
                    type_index,
                    callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                    callee_ty.returns().iter().map(|t| t.to_microwasm_type()),
                );
            }
        }
    }

    ctx.epilogue();

    mem::replace(&mut session.op_offset_map, op_offset_map);

    Ok(())
}
