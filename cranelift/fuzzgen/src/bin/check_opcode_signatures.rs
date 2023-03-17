use cranelift::codegen::ir::{instructions::ResolvedConstraint, types::*, Opcode, Type};
use cranelift_fuzzgen::OPCODE_SIGNATURES;
use std::collections::HashMap;

/// The list of opcodes that we exclude from the analysis. This should only be instructions that
/// affect control flow, as everything else should be handled by the general machinery in
/// function_generator.
const SKIPPED_OPCODES: &[Opcode] = &[
    // These opcodes have special handling in cranelift-fuzzgen
    Opcode::Call,
    Opcode::Return,
    Opcode::Jump,
    Opcode::Brif,
    Opcode::BrTable,
    // TODO: ExtractVector produces dynamic vectors, and those cause a panic in
    // `constraint.result_type`.
    Opcode::ExtractVector,
];

/// This is the set of types that we know how to fuzz in cranelift. It's not specialized by
/// targets, as we expect any target-specific support for things like SIMD to be expressed in the
/// `function_generator::valid_for_target` predicate instead.
const TYPES: &[Type] = &[
    I8, I16, I32, I64, I128, // Scalar Integers
    F32, F64, // Scalar Floats
    I8X16, I16X8, I32X4, I64X2, // SIMD Integers
    F32X4, F64X2, // SIMD Floats
];

fn instruction_instantiations<'a>() -> Vec<(Opcode, Vec<Type>, Vec<Type>)> {
    let mut insts = vec![];

    for op in Opcode::all() {
        if SKIPPED_OPCODES.contains(op) {
            continue;
        }

        let constraints = op.constraints();

        let ctrl_types = if let Some(ctrls) = constraints.ctrl_typeset() {
            Vec::from_iter(TYPES.iter().copied().filter(|ty| ctrls.contains(*ty)))
        } else {
            vec![INVALID]
        };

        for ctrl_type in ctrl_types {
            let rets = Vec::from_iter(
                (0..constraints.num_fixed_results()).map(|i| constraints.result_type(i, ctrl_type)),
            );

            let mut cols = vec![];

            for i in 0..constraints.num_fixed_value_arguments() {
                match constraints.value_argument_constraint(i, ctrl_type) {
                    ResolvedConstraint::Bound(ty) => cols.push(Vec::from([ty])),
                    ResolvedConstraint::Free(tys) => cols.push(Vec::from_iter(
                        TYPES.iter().copied().filter(|ty| tys.contains(*ty)),
                    )),
                }
            }

            let mut argss = vec![vec![]];

            let mut cols = cols.as_slice();
            while let Some((col, rest)) = cols.split_last() {
                cols = rest;

                let mut next = vec![];
                for current in argss.iter() {
                    for ty in col {
                        let mut args = vec![*ty];
                        args.extend_from_slice(&current);
                        next.push(args);
                    }
                }

                let _ = std::mem::replace(&mut argss, next);
            }

            for args in argss {
                insts.push((*op, args, rets.clone()));
            }
        }
    }

    insts
}

#[derive(Eq, PartialEq, Debug)]
struct Inst<'a> {
    args: &'a [Type],
    rets: &'a [Type],
}

fn build_sig_map<'a, T>(sigs: &'a [(Opcode, T, T)]) -> HashMap<Opcode, Vec<Inst<'a>>>
where
    T: AsRef<[Type]>,
{
    let mut insts = HashMap::<Opcode, Vec<Inst<'a>>>::default();

    for (op, args, rets) in sigs {
        insts.entry(*op).or_default().push(Inst {
            args: args.as_ref(),
            rets: rets.as_ref(),
        });
    }

    insts
}

fn main() {
    let all_ops = instruction_instantiations();
    let everything = build_sig_map(&all_ops);
    let fuzzed = build_sig_map(OPCODE_SIGNATURES);

    let mut unknown = vec![];
    for (op, insts) in fuzzed.iter() {
        if let Some(known_insts) = everything.get(op) {
            let invalid = Vec::from_iter(insts.iter().filter(|inst| !known_insts.contains(inst)));
            if !invalid.is_empty() {
                println!("# Invalid instantiations for Opcode::{:?}", op);
                for inst in invalid {
                    println!("- args: `{:?}`, rets: `{:?}`", inst.args, inst.rets);
                }
            }
        } else {
            unknown.push(*op);
        }
    }

    if !unknown.is_empty() {
        println!();
        println!("# Instructions without known instantiations");
        for op in unknown {
            println!("- Opcode::{:?}", op);
        }
    }
}
