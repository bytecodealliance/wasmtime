#[cfg(debug_assertions)]
use crate::backend::Registers;
use crate::backend::{
    ret_locs, BlockCallingConvention, BrAction, CodeGenSession, Label,  Target,
    ValueLocation, VirtualCallingConvention,
};
use crate::{
    error::Error,
    microwasm::*,
    module::{ModuleContext, SigType, Signature},
};
use cranelift_codegen::{binemit, ir};
use dynasmrt::DynasmApi;
use itertools::{
    Either::{self, Left, Right},
    Itertools,
};
#[cfg(debug_assertions)]
use more_asserts::assert_ge;
use std::{collections::HashMap, fmt, hash::Hash, iter, mem};
use wasmparser::FunctionBody;

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

pub trait OffsetSink {
    fn offset(
        &mut self,
        offset_in_wasm_function: ir::SourceLoc,
        offset_in_compiled_function: usize,
    );
}

pub struct NullOffsetSink;

impl OffsetSink for NullOffsetSink {
    fn offset(&mut self, _: ir::SourceLoc, _: usize) {}
}

pub struct Sinks<'a> {
    pub relocs: &'a mut dyn binemit::RelocSink,
    pub traps: &'a mut dyn binemit::TrapSink,
    pub offsets: &'a mut dyn OffsetSink,
}

impl Sinks<'_> {
    pub fn reborrow<'a>(&'a mut self) -> Sinks<'a> {
        Sinks {
            relocs: &mut *self.relocs,
            traps: &mut *self.traps,
            offsets: &mut *self.offsets,
        }
    }
}

pub fn translate_wasm<M>(
    session: &mut CodeGenSession<M>,
    sinks: Sinks<'_>,
    func_idx: u32,
    body: FunctionBody<'_>,
) -> Result<(), Error>
where
    M: ModuleContext,
    for<'any> &'any M::Signature: Into<OpSig>,
{
    let ty = session.module_context.defined_func_type(func_idx);

    let microwasm_conv = MicrowasmConv::new(
        session.module_context,
        ty.params().iter().map(SigType::to_microwasm_type),
        ty.returns().iter().map(SigType::to_microwasm_type),
        body,
        session.pointer_type(),
    )?
    .flat_map(|ops| match ops {
        Ok(ops) => Left(ops.into_iter().map(Ok)),
        Err(e) => Right(iter::once(Err(Error::Microwasm(e.to_string())))),
    });

    translate(session, sinks, func_idx, microwasm_conv)?;
    Ok(())
}

pub fn translate<M, I, L: Send + Sync + 'static>(
    session: &mut CodeGenSession<M>,
    mut sinks: Sinks<'_>,
    func_idx: u32,
    body: I,
) -> Result<(), Error>
where
    M: ModuleContext,
    I: IntoIterator<Item = Result<WithLoc<Operator<L>>, Error>>,
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
    {
        let ctx = &mut session.new_context(func_idx, sinks.reborrow());
        op_offset_map.push((
            ctx.asm.offset(),
            Box::new(format!("Function {}:", func_idx)),
        ));

        let params = func_type
            .params()
            .iter()
            .map(|t| t.to_microwasm_type())
            .collect::<Vec<_>>();

        ctx.start_function(params.iter().cloned())?;

        let mut blocks = HashMap::<BrTarget<L>, Block>::new();

        let num_returns = func_type.returns().len();

        let loc = ret_locs(func_type.returns().iter().map(|t| t.to_microwasm_type()))?;

        blocks.insert(
            BrTarget::Return,
            Block {
                label: BrTarget::Return,
                params: num_returns as u32,
                calling_convention: Some(Left(BlockCallingConvention::function_start(loc))),
                is_next: false,
                has_backwards_callers: false,
                actual_num_callers: 0,
                num_callers: None,
            },
        );

        let mut in_block = true;

        while let Some(op_offset) = body.next() {
            let WithLoc { op, offset } = op_offset?;

            ctx.set_source_loc(offset);
            ctx.sinks.offsets.offset(offset, ctx.asm.offset().0);

            if let Some(Ok(WithLoc {
                op: Operator::Start(label),
                ..
            })) = body.peek()
            {
                let block = match blocks.get_mut(&BrTarget::Label(label.clone())) {
                    None => {
                        return Err(Error::Microwasm(
                            "Label defined before being declared".to_string(),
                        ))
                    }
                    Some(o) => o,
                };
                block.is_next = true;
            }

            // `cfg` on blocks doesn't work in the compiler right now, so we have to write a dummy macro
            #[cfg(debug_assertions)]
            macro_rules! assertions {
                () => {
                    if let Operator::Start(label) = &op {
                        debug_assert!(!in_block);

                        let block = &blocks[&BrTarget::Label(label.clone())];
                        let num_cc_params = block.calling_convention.as_ref().map(|cc| match cc {
                            Left(cc) => cc.arguments.len(),
                            Right(cc) => cc.stack.len(),
                        });
                        if let Some(num_cc_params) = num_cc_params {
                            // we can use assert here bc we are in debug mode
                            assert_ge!(num_cc_params, block.params as usize);
                        }
                    } else {
                        debug_assert!(in_block);

                        let mut actual_regs = Registers::new();
                        actual_regs.release_scratch_register()?;
                        for val in &ctx.block_state.stack {
                            if let ValueLocation::Reg(gpr) = val {
                                actual_regs.mark_used(*gpr);
                            }
                        }
                        // we can use assert here bc we are in debug mode
                        assert_eq!(actual_regs, ctx.block_state.regs);
                    }
                };
            }

            #[cfg(not(debug_assertions))]
            macro_rules! assertions {
                () => {};
            }

            println!("{}", op);

            assertions!();

            struct DisassemblyOpFormatter<Label>(WithLoc<Operator<Label>>);

            impl<Label> fmt::Display for DisassemblyOpFormatter<Label>
            where
                Operator<Label>: fmt::Display,
            {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    match &self.0 {
                        WithLoc {
                            op: op @ Operator::Start(_),
                            ..
                        } => write!(f, "{}", op),
                        WithLoc {
                            op: op @ Operator::Declare { .. },
                            ..
                        } => write!(f, "{:5}\t{}", "", op),
                        WithLoc { op, offset } => {
                            if offset.is_default() {
                                write!(f, "{:5}\t  {}", "", op)
                            } else {
                                write!(f, "{:5}\t  {}", offset, op)
                            }
                        }
                    }
                }
            }

            op_offset_map.push((
                ctx.asm.offset(),
                Box::new(DisassemblyOpFormatter(WithLoc {
                    op: op.clone(),
                    offset,
                })),
            ));

            if let Operator::Start(_) = &op {
                if in_block {
                    return Err(Error::Microwasm(
                        "New block started without previous block ending".to_string(),
                    ));
                }
            } else {
                if !in_block {
                    return Err(Error::Microwasm("Operator not in block".to_string()));
                }
            }

            match op {
                Operator::Unreachable => {
                    in_block = false;

                    ctx.trap(ir::TrapCode::UnreachableCodeReached);
                }
                Operator::Start(label) => {
                    use std::collections::hash_map::Entry;

                    in_block = true;

                    if let Entry::Occupied(mut entry) = blocks.entry(BrTarget::Label(label.clone()))
                    {
                        let has_backwards_callers = {
                            let block = entry.get_mut();

                            // TODO: Maybe we want to restrict Microwasm so that at least one of its callers
                            //       must be before the label. In an ideal world the restriction would be that
                            //       blocks without callers are illegal, but that's not reasonably possible for
                            //       Microwasm generated from Wasm.
                            if block.actual_num_callers == 0 {
                                if block.calling_convention.is_some() {
                                    return Err(Error::Microwasm(
                                        "Block marked unreachable but has been jumped to before"
                                            .to_string(),
                                    ));
                                }

                                loop {
                                    let WithLoc { op: skipped, .. } = if let Some(op) = body.next()
                                    {
                                        op
                                    } else {
                                        break;
                                    }?;

                                    match skipped {
                                        Operator::End(..) => {
                                            in_block = false;
                                            break;
                                        }
                                        Operator::Start(..) => {
                                            return Err(Error::Microwasm(
                                                "New block started without previous block ending"
                                                    .to_string(),
                                            ));
                                        }
                                        Operator::Declare {
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
                                        _ => {}
                                    }
                                }

                                continue;
                            }

                            block.is_next = false;

                            // TODO: We can `take` this if it's a `Right`
                            match block.calling_convention.as_ref() {
                                Some(Left(cc)) => {
                                    ctx.apply_cc(cc.as_ref())?;
                                }
                                Some(Right(virt)) => {
                                    ctx.set_state(virt.clone())?;
                                }
                                None => {
                                    return Err(Error::Microwasm(
                                        "No calling convention to apply".to_string(),
                                    ));
                                }
                            }

                            ctx.define_label(block.label.label().unwrap().clone());

                            ctx.shrink_stack_to_fit()?;

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
                Operator::Declare {
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
                Operator::End(Targets { targets, default }) => {
                    in_block = false;

                    let target_labels = targets.iter().map(|t| {
                        let block = &blocks[&t.target];
                        Target {
                            target: block.label,
                            action: if block.is_next {
                                BrAction::Continue
                            } else {
                                BrAction::Jump
                            }
                        }
                    }).collect::<Vec<_>>();
                    let default_label = {
                        let block = &blocks[&default.target];
                        Target {
                            target: block.label,
                            action: if block.is_next {
                                BrAction::Continue
                            } else {
                                BrAction::Jump
                            }
                        }
                    };

                    ctx.end_block(target_labels, default_label, |ctx, mut selector| {
                        let (mut cc_and_to_drop, mut max_num_callers, params) = {
                            let def = blocks.get_mut(&default.target).unwrap();
    
                            def.actual_num_callers = def.actual_num_callers.saturating_add(1);
    
                            (
                                def.calling_convention.clone().map(|cc| (cc, default.to_drop.clone())),
                                def.num_callers,
                                def.params
                                    + default
                                        .to_drop
                                        .as_ref()
                                        .map(|t| t.clone().count())
                                        .unwrap_or_default() as u32,
                            )
                        };

                        for target in targets.iter().unique() {
                            let block = blocks.get_mut(&target.target).unwrap();
                            // Although performance is slightly worse if we miscount this to be too high,
                            // it doesn't affect correctness.
                            block.actual_num_callers = block.actual_num_callers.saturating_add(1);

                            if let Some(block_cc) = &block.calling_convention {
                                if let Some((cc, to_drop)) = &cc_and_to_drop {
                                    if cc != block_cc || to_drop != &target.to_drop {
                                        return Err(Error::Microwasm(
                                            "Can't pass different params to different elements of `br_table` yet"
                                                .to_string()));
                                    }
                                }

                                cc_and_to_drop = Some((block_cc.clone(), target.to_drop.clone()));
                            }

                            if let Some(max) = max_num_callers {
                                max_num_callers = block.num_callers.map(|n| max.max(n));
                            }

                            debug_assert_eq!(
                                params,
                                block.params
                                    + target
                                        .to_drop
                                        .as_ref()
                                        .map(|t| t.clone().count())
                                        .unwrap_or_default() as u32,
                            );
                        }

                        let temp: Result<
                            Either<BlockCallingConvention, VirtualCallingConvention>,
                            Error,
                        > = cc_and_to_drop
                            .map(|(cc, to_drop)| match cc {
                                Left(cc) => {
                                    if cc.arguments.iter().any(|loc| ValueLocation::from(*loc) == selector) {
                                        selector = ctx.push_physical(selector)?.into();
                                    }

                                    let (end, count) = to_drop.as_ref().map(|to_drop| {
                                        (*to_drop.end() as usize, to_drop.clone().count())
                                    }).unwrap_or_default();
                                    let end = cc.arguments.len().saturating_sub(end + 1);

                                    let extra = (params as usize)
                                        .checked_sub(cc.arguments.len() + count)
                                        .unwrap();

                                    let locs = std::iter::repeat(None)
                                        .take(extra)
                                        .chain(cc.arguments[..end].iter()
                                        .cloned()
                                        .map(Some))
                                        .chain(std::iter::repeat(None).take(count))
                                        .chain(
                                            cc.arguments[end..].iter()
                                                .cloned()
                                                .map(Some)
                                        )
                                        .collect::<Vec<_>>();

                                    let tmp = ctx.serialize_block_args(locs, cc.stack_depth)?;
                                    Ok(Left(tmp))
                                }
                                Right(cc) => Ok(Right(cc)),
                            })
                            .unwrap_or_else(|| {
                                if max_num_callers.map(|callers| callers <= 1).unwrap_or(false) {
                                    Ok(Right(ctx.virtual_calling_convention()))
                                } else {
                                    let tmp = ctx.serialize_args(params)?;
                                    Ok(Left(tmp))
                                }
                            });
                        let cc = match temp.unwrap() {
                            Right(rr) => Right(rr),
                            Left(l) => Left(l),
                        };

                        for target in targets.iter().chain(std::iter::once(&default)).unique() {
                            let block = blocks.get_mut(&target.target).unwrap();
                            let mut cc = cc.clone();
                            if let Some(to_drop) = target.to_drop.clone() {
                                match &mut cc {
                                    Left(cc) => drop_elements(&mut cc.arguments, to_drop),
                                    Right(cc) => drop_elements(&mut cc.stack, to_drop),
                                }
                            }

                            debug_assert_eq!(
                                match &cc {
                                    Left(cc) => cc.arguments.len(),
                                    Right(cc) => cc.stack.len(),
                                },
                                block.params as usize,
                            );

                            block.calling_convention = Some(cc);
                        }

                        Ok((
                            selector,
                            match &cc {
                                Left(cc) => cc.stack_depth.clone(),
                                Right(cc) => cc.depth.clone(),
                            }
                        ))
                    })?;
                }
                Operator::Swap(depth) => ctx.swap(depth),
                Operator::Pick(depth) => ctx.pick(depth),
                Operator::Eq(I32) => ctx.i32_eq()?,
                Operator::Eqz(Size::_32) => ctx.i32_eqz()?,
                Operator::Ne(I32) => ctx.i32_neq()?,
                Operator::Lt(SI32) => ctx.i32_lt_s()?,
                Operator::Le(SI32) => ctx.i32_le_s()?,
                Operator::Gt(SI32) => ctx.i32_gt_s()?,
                Operator::Ge(SI32) => ctx.i32_ge_s()?,
                Operator::Lt(SU32) => ctx.i32_lt_u()?,
                Operator::Le(SU32) => ctx.i32_le_u()?,
                Operator::Gt(SU32) => ctx.i32_gt_u()?,
                Operator::Ge(SU32) => ctx.i32_ge_u()?,
                Operator::Add(I32) => ctx.i32_add()?,
                Operator::Sub(I32) => ctx.i32_sub()?,
                Operator::And(Size::_32) => ctx.i32_and()?,
                Operator::Or(Size::_32) => ctx.i32_or()?,
                Operator::Xor(Size::_32) => ctx.i32_xor()?,
                Operator::Mul(I32) => ctx.i32_mul()?,
                Operator::Div(SU32) => ctx.i32_div_u()?,
                Operator::Div(SI32) => ctx.i32_div_s()?,
                Operator::Rem(sint::I32) => ctx.i32_rem_s()?,
                Operator::Rem(sint::U32) => ctx.i32_rem_u()?,
                Operator::Shl(Size::_32) => ctx.i32_shl()?,
                Operator::Shr(sint::I32) => ctx.i32_shr_s()?,
                Operator::Shr(sint::U32) => ctx.i32_shr_u()?,
                Operator::Rotl(Size::_32) => ctx.i32_rotl()?,
                Operator::Rotr(Size::_32) => ctx.i32_rotr()?,
                Operator::Clz(Size::_32) => ctx.i32_clz()?,
                Operator::Ctz(Size::_32) => ctx.i32_ctz()?,
                Operator::Popcnt(Size::_32) => ctx.i32_popcnt()?,
                Operator::Eq(I64) => ctx.i64_eq()?,
                Operator::Eqz(Size::_64) => ctx.i64_eqz()?,
                Operator::Ne(I64) => ctx.i64_neq()?,
                Operator::Lt(SI64) => ctx.i64_lt_s()?,
                Operator::Le(SI64) => ctx.i64_le_s()?,
                Operator::Gt(SI64) => ctx.i64_gt_s()?,
                Operator::Ge(SI64) => ctx.i64_ge_s()?,
                Operator::Lt(SU64) => ctx.i64_lt_u()?,
                Operator::Le(SU64) => ctx.i64_le_u()?,
                Operator::Gt(SU64) => ctx.i64_gt_u()?,
                Operator::Ge(SU64) => ctx.i64_ge_u()?,
                Operator::Add(I64) => ctx.i64_add()?,
                Operator::Sub(I64) => ctx.i64_sub()?,
                Operator::And(Size::_64) => ctx.i64_and()?,
                Operator::Or(Size::_64) => ctx.i64_or()?,
                Operator::Xor(Size::_64) => ctx.i64_xor()?,
                Operator::Mul(I64) => ctx.i64_mul()?,
                Operator::Div(SU64) => ctx.i64_div_u()?,
                Operator::Div(SI64) => ctx.i64_div_s()?,
                Operator::Rem(sint::I64) => ctx.i64_rem_s()?,
                Operator::Rem(sint::U64) => ctx.i64_rem_u()?,
                Operator::Shl(Size::_64) => ctx.i64_shl()?,
                Operator::Shr(sint::I64) => ctx.i64_shr_s()?,
                Operator::Shr(sint::U64) => ctx.i64_shr_u()?,
                Operator::Rotl(Size::_64) => ctx.i64_rotl()?,
                Operator::Rotr(Size::_64) => ctx.i64_rotr()?,
                Operator::Clz(Size::_64) => ctx.i64_clz()?,
                Operator::Ctz(Size::_64) => ctx.i64_ctz()?,
                Operator::Popcnt(Size::_64) => ctx.i64_popcnt()?,
                Operator::Add(F32) => ctx.f32_add()?,
                Operator::Mul(F32) => ctx.f32_mul()?,
                Operator::Sub(F32) => ctx.f32_sub()?,
                Operator::Div(SF32) => ctx.f32_div()?,
                Operator::Min(Size::_32) => ctx.f32_min()?,
                Operator::Max(Size::_32) => ctx.f32_max()?,
                Operator::Copysign(Size::_32) => ctx.f32_copysign()?,
                Operator::Sqrt(Size::_32) => ctx.f32_sqrt()?,
                Operator::Neg(Size::_32) => ctx.f32_neg()?,
                Operator::Abs(Size::_32) => ctx.f32_abs()?,
                Operator::Floor(Size::_32) => ctx.f32_floor()?,
                Operator::Ceil(Size::_32) => ctx.f32_ceil()?,
                Operator::Nearest(Size::_32) => ctx.f32_nearest()?,
                Operator::Trunc(Size::_32) => ctx.f32_trunc()?,
                Operator::Eq(F32) => ctx.f32_eq()?,
                Operator::Ne(F32) => ctx.f32_ne()?,
                Operator::Gt(SF32) => ctx.f32_gt()?,
                Operator::Ge(SF32) => ctx.f32_ge()?,
                Operator::Lt(SF32) => ctx.f32_lt()?,
                Operator::Le(SF32) => ctx.f32_le()?,
                Operator::Add(F64) => ctx.f64_add()?,
                Operator::Mul(F64) => ctx.f64_mul()?,
                Operator::Sub(F64) => ctx.f64_sub()?,
                Operator::Div(SF64) => ctx.f64_div()?,
                Operator::Min(Size::_64) => ctx.f64_min()?,
                Operator::Max(Size::_64) => ctx.f64_max()?,
                Operator::Copysign(Size::_64) => ctx.f64_copysign()?,
                Operator::Sqrt(Size::_64) => ctx.f64_sqrt()?,
                Operator::Neg(Size::_64) => ctx.f64_neg()?,
                Operator::Abs(Size::_64) => ctx.f64_abs()?,
                Operator::Floor(Size::_64) => ctx.f64_floor()?,
                Operator::Ceil(Size::_64) => ctx.f64_ceil()?,
                Operator::Nearest(Size::_64) => ctx.f64_nearest()?,
                Operator::Trunc(Size::_64) => ctx.f64_trunc()?,
                Operator::Eq(F64) => ctx.f64_eq()?,
                Operator::Ne(F64) => ctx.f64_ne()?,
                Operator::Gt(SF64) => ctx.f64_gt()?,
                Operator::Ge(SF64) => ctx.f64_ge()?,
                Operator::Lt(SF64) => ctx.f64_lt()?,
                Operator::Le(SF64) => ctx.f64_le()?,
                Operator::Drop(range) => ctx.drop(range)?,
                Operator::Const(val) => ctx.const_(val)?,
                Operator::I32WrapFromI64 => ctx.i32_wrap_from_i64()?,
                Operator::I32ReinterpretFromF32 => ctx.i32_reinterpret_from_f32()?,
                Operator::I64ReinterpretFromF64 => ctx.i64_reinterpret_from_f64()?,
                Operator::F32ReinterpretFromI32 => ctx.f32_reinterpret_from_i32()?,
                Operator::F64ReinterpretFromI64 => ctx.f64_reinterpret_from_i64()?,
                Operator::ITruncFromF {
                    input_ty: Size::_32,
                    output_ty: sint::I32,
                } => {
                    ctx.i32_truncate_f32_s()?;
                }
                Operator::ITruncFromF {
                    input_ty: Size::_32,
                    output_ty: sint::U32,
                } => {
                    ctx.i32_truncate_f32_u()?;
                }
                Operator::ITruncFromF {
                    input_ty: Size::_64,
                    output_ty: sint::I32,
                } => {
                    ctx.i32_truncate_f64_s()?;
                }
                Operator::ITruncFromF {
                    input_ty: Size::_64,
                    output_ty: sint::U32,
                } => {
                    ctx.i32_truncate_f64_u()?;
                }
                Operator::ITruncFromF {
                    input_ty: Size::_32,
                    output_ty: sint::I64,
                } => {
                    ctx.i64_truncate_f32_s()?;
                }
                Operator::ITruncFromF {
                    input_ty: Size::_32,
                    output_ty: sint::U64,
                } => {
                    ctx.i64_truncate_f32_u()?;
                }
                Operator::ITruncFromF {
                    input_ty: Size::_64,
                    output_ty: sint::I64,
                } => {
                    ctx.i64_truncate_f64_s()?;
                }
                Operator::ITruncFromF {
                    input_ty: Size::_64,
                    output_ty: sint::U64,
                } => {
                    ctx.i64_truncate_f64_u()?;
                }
                Operator::Extend {
                    sign: Signedness::Unsigned,
                } => ctx.i32_extend_u()?,
                Operator::Extend {
                    sign: Signedness::Signed,
                } => ctx.i32_extend_s()?,
                Operator::FConvertFromI {
                    input_ty: sint::I32,
                    output_ty: Size::_32,
                } => ctx.f32_convert_from_i32_s()?,
                Operator::FConvertFromI {
                    input_ty: sint::I32,
                    output_ty: Size::_64,
                } => ctx.f64_convert_from_i32_s()?,
                Operator::FConvertFromI {
                    input_ty: sint::I64,
                    output_ty: Size::_32,
                } => ctx.f32_convert_from_i64_s()?,
                Operator::FConvertFromI {
                    input_ty: sint::I64,
                    output_ty: Size::_64,
                } => ctx.f64_convert_from_i64_s()?,
                Operator::FConvertFromI {
                    input_ty: sint::U32,
                    output_ty: Size::_32,
                } => ctx.f32_convert_from_i32_u()?,
                Operator::FConvertFromI {
                    input_ty: sint::U32,
                    output_ty: Size::_64,
                } => ctx.f64_convert_from_i32_u()?,
                Operator::FConvertFromI {
                    input_ty: sint::U64,
                    output_ty: Size::_32,
                } => ctx.f32_convert_from_i64_u()?,
                Operator::FConvertFromI {
                    input_ty: sint::U64,
                    output_ty: Size::_64,
                } => ctx.f64_convert_from_i64_u()?,
                Operator::F64PromoteFromF32 => ctx.f64_from_f32()?,
                Operator::F32DemoteFromF64 => ctx.f32_from_f64()?,
                Operator::Load8 {
                    ty: sint::U32,
                    memarg,
                } => ctx.i32_load8_u(memarg.offset)?,
                Operator::Load16 {
                    ty: sint::U32,
                    memarg,
                } => ctx.i32_load16_u(memarg.offset)?,
                Operator::Load8 {
                    ty: sint::I32,
                    memarg,
                } => ctx.i32_load8_s(memarg.offset)?,
                Operator::Load16 {
                    ty: sint::I32,
                    memarg,
                } => ctx.i32_load16_s(memarg.offset)?,
                Operator::Load8 {
                    ty: sint::U64,
                    memarg,
                } => ctx.i64_load8_u(memarg.offset)?,
                Operator::Load16 {
                    ty: sint::U64,
                    memarg,
                } => ctx.i64_load16_u(memarg.offset)?,
                Operator::Load8 {
                    ty: sint::I64,
                    memarg,
                } => ctx.i64_load8_s(memarg.offset)?,
                Operator::Load16 {
                    ty: sint::I64,
                    memarg,
                } => ctx.i64_load16_s(memarg.offset)?,
                Operator::Load32 {
                    sign: Signedness::Unsigned,
                    memarg,
                } => ctx.i64_load32_u(memarg.offset)?,
                Operator::Load32 {
                    sign: Signedness::Signed,
                    memarg,
                } => ctx.i64_load32_s(memarg.offset)?,
                Operator::Load { ty: I32, memarg } => ctx.i32_load(memarg.offset)?,
                Operator::Load { ty: F32, memarg } => ctx.f32_load(memarg.offset)?,
                Operator::Load { ty: I64, memarg } => ctx.i64_load(memarg.offset)?,
                Operator::Load { ty: F64, memarg } => ctx.f64_load(memarg.offset)?,
                Operator::Store8 { memarg, .. } => ctx.store8(memarg.offset)?,
                Operator::Store16 { memarg, .. } => ctx.store16(memarg.offset)?,
                Operator::Store32 { memarg }
                | Operator::Store { ty: I32, memarg }
                | Operator::Store { ty: F32, memarg } => ctx.store32(memarg.offset)?,
                Operator::Store { ty: I64, memarg } | Operator::Store { ty: F64, memarg } => {
                    ctx.store64(memarg.offset)?
                }
                Operator::GlobalGet(idx) => ctx.get_global(idx)?,
                Operator::GlobalSet(idx) => ctx.set_global(idx)?,
                Operator::Select => {
                    ctx.select()?;
                }
                Operator::MemorySize { .. } => {
                    ctx.memory_size()?;
                }
                Operator::MemoryGrow { .. } => {
                    ctx.memory_grow()?;
                }
                Operator::Call { function_index } => {
                    let callee_ty = module_context.func_type(function_index);

                    if let Some(defined_index) = module_context.defined_func_index(function_index) {
                        if defined_index == func_idx {
                            ctx.call_direct_self(
                                callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                                callee_ty.returns().iter().map(|t| t.to_microwasm_type()),
                            )?;
                        } else {
                            ctx.call_direct(
                                function_index,
                                callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                                callee_ty.returns().iter().map(|t| t.to_microwasm_type()),
                            )?;
                        }
                    } else {
                        ctx.call_direct_imported(
                            function_index,
                            callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                            callee_ty.returns().iter().map(|t| t.to_microwasm_type()),
                        )?;
                    }
                }
                Operator::CallIndirect {
                    type_index,
                    table_index,
                } => {
                    if table_index != 0 {
                        return Err(Error::Microwasm("table_index not equal to 0".to_string()));
                    }

                    let callee_ty = module_context.signature(type_index);

                    ctx.call_indirect(
                        type_index,
                        callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                        callee_ty.returns().iter().map(|t| t.to_microwasm_type()),
                    )?;
                }
            }
        }

        ctx.epilogue();
    }

    mem::replace(&mut session.op_offset_map, op_offset_map);

    Ok(())
}
