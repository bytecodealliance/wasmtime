use crate::backend::*;
use crate::error::Error;
use crate::microwasm::*;
use crate::module::{quickhash, ModuleContext, SigType, Signature};
use either::{Either, Left, Right};
use multi_mut::HashMapMultiMut;
use std::{collections::HashMap, convert::TryInto, hash::Hash};

#[derive(Debug)]
struct Block {
    label: BrTarget<Label>,
    calling_convention: Option<Either<CallingConvention, VirtualCallingConvention>>,
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

pub fn translate_wasm<M: ModuleContext>(
    session: &mut CodeGenSession<M>,
    func_idx: u32,
    body: &wasmparser::FunctionBody,
) -> Result<(), Error>
where
    for<'any> &'any M::Signature: Into<OpSig>,
{
    let ty = session.module_context.func_type(func_idx);

    if DISASSEMBLE {
        let mut microwasm = vec![];

        let microwasm_conv = MicrowasmConv::new(
            session.module_context,
            ty.params().iter().map(SigType::to_microwasm_type),
            ty.returns().iter().map(SigType::to_microwasm_type),
            body,
        );

        for ops in microwasm_conv {
            microwasm.extend(ops?);
        }

        println!("{}", crate::microwasm::dis(func_idx, &microwasm));
    }

    let microwasm_conv = MicrowasmConv::new(
        session.module_context,
        ty.params().iter().map(SigType::to_microwasm_type),
        ty.returns().iter().map(SigType::to_microwasm_type),
        body,
    );

    translate(
        session,
        func_idx,
        microwasm_conv.flat_map(|i| i.expect("TODO: Make this not panic")),
    )
}

pub fn translate<M: ModuleContext, I, L>(
    session: &mut CodeGenSession<M>,
    func_idx: u32,
    body: I,
) -> Result<(), Error>
where
    I: IntoIterator<Item = Operator<L>>,
    L: Hash + Clone + Eq,
    Operator<L>: std::fmt::Display,
{
    let func_type = session.module_context.defined_func_type(func_idx);
    let mut body = body.into_iter().peekable();

    let ctx = &mut session.new_context(func_idx);

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
            // TODO: This only works for integers
            //
            calling_convention: Some(Left(CallingConvention::function_start(ret_locs(
                func_type.returns().iter().map(|t| t.to_microwasm_type()),
            )))),
            is_next: false,
            has_backwards_callers: false,
            actual_num_callers: 0,
            num_callers: None,
        },
    );

    loop {
        let op = if let Some(op) = body.next() {
            op
        } else {
            break;
        };

        if let Some(Operator::Label(label)) = body.peek() {
            let block = blocks
                .get_mut(&BrTarget::Label(label.clone()))
                .expect("Block definition should be before label definition");
            block.is_next = true;
        }

        match op {
            Operator::Unreachable => {
                ctx.trap();
            }
            Operator::Label(label) => {
                use std::collections::hash_map::Entry;

                if let Entry::Occupied(mut entry) = blocks.entry(BrTarget::Label(label)) {
                    let has_backwards_callers = {
                        let block = entry.get_mut();

                        // TODO: Is it possible with arbitrary CFGs that a block will have _only_ backwards callers?
                        //       Certainly for Microwasm generated from Wasm that is currently impossible.
                        if block.actual_num_callers == 0 {
                            loop {
                                let done = match body.peek() {
                                    Some(Operator::Label(_)) | None => true,
                                    Some(_) => false,
                                };

                                if done {
                                    break;
                                }

                                body.next();
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
                            _ => {}
                        }

                        ctx.define_label(block.label.label().unwrap().clone());

                        block.has_backwards_callers
                    };

                    // To reduce memory overhead
                    if !has_backwards_callers {
                        entry.remove_entry();
                    }
                } else {
                    panic!("Label defined before being declared");
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
                let (then_block, else_block) = blocks.pair_mut(&then, &else_);
                // TODO: If actual_num_callers == num_callers then we can remove this block from the hashmap.
                //       This frees memory and acts as a kind of verification that `num_callers` is set
                //       correctly. It doesn't help for loops and block ends generated from Wasm.
                then_block.actual_num_callers += 1;
                else_block.actual_num_callers += 1;

                let then_block_parts = (then_block.is_next, then_block.label);
                let else_block_parts = (else_block.is_next, else_block.label);

                // TODO: Use "compatible" cc
                assert_eq!(then_block.params, else_block.params);

                // TODO: The blocks should have compatible (one must be subset of other?) calling
                //       conventions or else at least one must have no calling convention. This
                //       should always be true for converting from WebAssembly AIUI.
                let f = |ctx: &mut Context<_>| {
                    let then_block_should_serialize_args = then_block.should_serialize_args();
                    let else_block_should_serialize_args = else_block.should_serialize_args();

                    match (
                        &mut then_block.calling_convention,
                        &mut else_block.calling_convention,
                    ) {
                        (Some(Left(ref cc)), ref mut other @ None)
                        | (ref mut other @ None, Some(Left(ref cc))) => {
                            **other = Some(Left(cc.clone()));

                            ctx.pass_block_args(cc);
                        }
                        (ref mut then_cc @ None, ref mut else_cc @ None) => {
                            let cc = if then_block_should_serialize_args {
                                Some(Left(ctx.serialize_args(then_block.params)))
                            } else if else_block_should_serialize_args {
                                Some(Left(ctx.serialize_args(else_block.params)))
                            } else {
                                Some(Right(ctx.virtual_calling_convention()))
                            };

                            **then_cc = cc.clone();
                            **else_cc = cc;
                        }
                        _ => unimplemented!(
                            "Can't pass different params to different sides of `br_if` yet"
                        ),
                    }
                };

                match (then_block_parts, else_block_parts) {
                    ((true, _), (false, BrTarget::Label(else_))) => {
                        ctx.br_if_false(else_, f);
                    }
                    ((false, BrTarget::Label(then)), (true, _)) => {
                        ctx.br_if_true(then, f);
                    }
                    ((false, BrTarget::Label(then)), (false, BrTarget::Label(else_))) => {
                        ctx.br_if_true(then, f);
                        ctx.br(else_);
                    }
                    other => unimplemented!("{:#?}", other),
                }
            }
            Operator::BrTable(BrTable { targets, default }) => {
                use itertools::Itertools;

                let (def, params) = {
                    let def = blocks.get(&default).unwrap();
                    (
                        if def.is_next {
                            None
                        } else {
                            Some(def.label)
                        },
                        def.params.clone()
                    )
                };

                let target_labels = targets.iter()
                    .map(|target| blocks.get(target).unwrap().label)
                    .collect::<Vec<_>>();

                ctx.br_table(target_labels, def, |ctx| {
                    let mut cc = None;
                    let mut max_num_callers = Some(0);

                    for target in targets.iter().chain(std::iter::once(&default)).unique() {
                        let block = blocks.get_mut(target).unwrap();
                        block.actual_num_callers += 1;

                        if block.calling_convention.is_some() {
                            assert!(cc.is_none(), "Can't pass different params to different elements of `br_table` yet");
                            cc = block.calling_convention.clone();
                        }

                        if let Some(max) = max_num_callers {
                            max_num_callers = block.num_callers.map(|n| max.max(n));
                        }
                    }

                    if let Some(Left(cc)) = &cc {
                        ctx.pass_block_args(cc);
                    }
       
                    let cc = cc.unwrap_or_else(||
                        if max_num_callers == Some(1) {
                            Right(ctx.virtual_calling_convention())
                        } else {
                            Left(ctx.serialize_args(params))
                        }
                    );

                    for target in targets.iter().chain(std::iter::once(&default)).unique() {
                        let block = blocks.get_mut(target).unwrap();
                        block.calling_convention = Some(cc.clone());
                    }
                });
            }
            Operator::Swap { depth } => ctx.swap(depth),
            Operator::Pick { depth } => ctx.pick(depth),
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
            Operator::Rem(sint::I32) => ctx.i32_rem_u(),
            Operator::Rem(sint::U32) => ctx.i32_rem_s(),
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
            Operator::Neg(Size::_32) => ctx.f32_neg(),
            Operator::Gt(SF32) => ctx.f32_gt(),
            Operator::Ge(SF32) => ctx.f32_ge(),
            Operator::Lt(SF32) => ctx.f32_lt(),
            Operator::Le(SF32) => ctx.f32_le(),
            Operator::Add(F64) => ctx.f64_add(),
            Operator::Mul(F64) => ctx.f64_mul(),
            Operator::Sub(F64) => ctx.f64_sub(),
            Operator::Neg(Size::_64) => ctx.f64_neg(),
            Operator::Gt(SF64) => ctx.f64_gt(),
            Operator::Ge(SF64) => ctx.f64_ge(),
            Operator::Lt(SF64) => ctx.f64_lt(),
            Operator::Le(SF64) => ctx.f64_le(),
            Operator::Drop(range) => ctx.drop(range),
            Operator::Const(val) => ctx.const_(val),
            Operator::Load { ty: I32, memarg } => ctx.i32_load(memarg.offset)?,
            Operator::Load { ty: I64, memarg } => ctx.i64_load(memarg.offset)?,
            Operator::Store { ty: I32, memarg } => ctx.i32_store(memarg.offset)?,
            Operator::Store { ty: I64, memarg } => ctx.i64_store(memarg.offset)?,
            Operator::Select => {
                ctx.select();
            }
            Operator::Call { function_index } => {
                let function_index = session
                    .module_context
                    .defined_func_index(function_index)
                    .expect("We don't support host calls yet");
                let callee_ty = session.module_context.func_type(function_index);

                // TODO: this implementation assumes that this function is locally defined.
                ctx.call_direct(
                    function_index,
                    callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                    callee_ty.returns().len() as u32,
                );
            }
            Operator::CallIndirect {
                type_index,
                table_index,
            } => {
                assert_eq!(table_index, 0);

                let callee_ty = session.module_context.signature(type_index);

                // TODO: this implementation assumes that this function is locally defined.

                ctx.call_indirect(
                    quickhash(callee_ty) as u32,
                    callee_ty.params().iter().map(|t| t.to_microwasm_type()),
                    callee_ty.returns().len() as u32,
                );
            }
            op => {
                unimplemented!("{}", op);
            }
        }
    }

    ctx.epilogue();

    Ok(())
}
