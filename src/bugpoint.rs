//! CLI tool to reduce Cranelift IR files crashing during compilation.

use crate::disasm::{PrintRelocs, PrintStackmaps, PrintTraps};
use crate::utils::{parse_sets_and_triple, read_to_string};
use cranelift_codegen::ir::{
    Ebb, FuncRef, Function, GlobalValueData, Inst, InstBuilder, InstructionData, StackSlots,
    TrapCode,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::Context;
use cranelift_entity::PrimaryMap;
use cranelift_reader::{parse_test, ParseOptions};
use std::collections::HashMap;
use std::path::Path;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

pub fn run(
    filename: &str,
    flag_set: &[String],
    flag_isa: &str,
    verbose: bool,
) -> Result<(), String> {
    let parsed = parse_sets_and_triple(flag_set, flag_isa)?;
    let fisa = parsed.as_fisa();

    let path = Path::new(&filename).to_path_buf();

    let buffer = read_to_string(&path).map_err(|e| format!("{}: {}", filename, e))?;
    let test_file =
        parse_test(&buffer, ParseOptions::default()).map_err(|e| format!("{}: {}", filename, e))?;

    // If we have an isa from the command-line, use that. Otherwise if the
    // file contains a unique isa, use that.
    let isa = if let Some(isa) = fisa.isa {
        isa
    } else if let Some(isa) = test_file.isa_spec.unique_isa() {
        isa
    } else {
        return Err(String::from("compilation requires a target isa"));
    };

    std::env::set_var("RUST_BACKTRACE", "0"); // Disable backtraces to reduce verbosity

    for (func, _) in test_file.functions {
        let (orig_ebb_count, orig_inst_count) = (ebb_count(&func), inst_count(&func));

        match reduce(isa, func, verbose) {
            Ok((func, crash_msg)) => {
                println!("Crash message: {}", crash_msg);

                println!("\n{}", func);

                println!(
                    "{} ebbs {} insts -> {} ebbs {} insts",
                    orig_ebb_count,
                    orig_inst_count,
                    ebb_count(&func),
                    inst_count(&func)
                );
            }
            Err(err) => println!("Warning: {}", err),
        }
    }

    Ok(())
}

enum MutationKind {
    /// The mutation reduced the amount of instructions or ebbs.
    Shrinked,
    /// The mutation only changed an instruction. Performing another round of mutations may only
    /// reduce the test case if another mutation shrank the test case.
    Changed,
}

trait Mutator {
    fn name(&self) -> &'static str;

    fn mutation_count(&self, func: &Function) -> Option<usize>;

    fn mutate(&mut self, func: Function) -> Option<(Function, String, MutationKind)>;

    fn reduce(
        &mut self,
        ccc: &mut CrashCheckContext,
        mut func: Function,
        progress_bar_prefix: String,
        verbose: bool,
        should_keep_reducing: &mut bool,
    ) -> Function {
        let progress = ProgressBar::with_draw_target(
            self.mutation_count(&func).unwrap_or(0) as u64,
            ProgressDrawTarget::stdout(),
        );
        progress.set_style(
            ProgressStyle::default_bar().template("{bar:60} {prefix:40} {pos:>4}/{len:>4} {msg}"),
        );

        progress.set_prefix(&(progress_bar_prefix + &format!(" phase {}", self.name())));

        for _ in 0..10000 {
            progress.inc(1);

            let (mutated_func, msg, mutation_kind) = match self.mutate(func.clone()) {
                Some(res) => res,
                None => {
                    break;
                }
            };

            progress.set_message(&msg);

            match ccc.check_for_crash(&mutated_func) {
                CheckResult::Succeed => {
                    // Shrinking didn't hit the problem anymore, discard changes.
                    continue;
                }
                CheckResult::Crash(_) => {
                    // Panic remained while shrinking, make changes definitive.
                    func = mutated_func;
                    match mutation_kind {
                        MutationKind::Shrinked => {
                            *should_keep_reducing = true;
                            if verbose {
                                progress.println(format!("{}: shrink", msg));
                            }
                        }
                        MutationKind::Changed => {
                            if verbose {
                                progress.println(format!("{}: changed", msg));
                            }
                        }
                    }
                }
            }
        }

        progress.set_message("done");
        progress.finish();

        func
    }
}

/// Try to remove instructions.
struct RemoveInst {
    ebb: Ebb,
    inst: Inst,
}

impl RemoveInst {
    fn new(func: &Function) -> Self {
        let first_ebb = func.layout.entry_block().unwrap();
        let first_inst = func.layout.first_inst(first_ebb).unwrap();
        Self {
            ebb: first_ebb,
            inst: first_inst,
        }
    }
}

impl Mutator for RemoveInst {
    fn name(&self) -> &'static str {
        "remove inst"
    }

    fn mutation_count(&self, func: &Function) -> Option<usize> {
        Some(inst_count(func))
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, MutationKind)> {
        if let Some((prev_ebb, prev_inst)) =
            next_inst_ret_prev(&func, &mut self.ebb, &mut self.inst)
        {
            func.layout.remove_inst(prev_inst);
            if func.layout.ebb_insts(prev_ebb).next().is_none() {
                // Make sure empty ebbs are removed, as `next_inst_ret_prev` depends on non empty ebbs
                func.layout.remove_ebb(prev_ebb);
                Some((
                    func,
                    format!("Remove inst {} and empty ebb {}", prev_inst, prev_ebb),
                    MutationKind::Shrinked,
                ))
            } else {
                Some((
                    func,
                    format!("Remove inst {}", prev_inst),
                    MutationKind::Shrinked,
                ))
            }
        } else {
            None
        }
    }
}

/// Try to replace instructions with `iconst`.
struct ReplaceInstWithIconst {
    ebb: Ebb,
    inst: Inst,
}

impl ReplaceInstWithIconst {
    fn new(func: &Function) -> Self {
        let first_ebb = func.layout.entry_block().unwrap();
        let first_inst = func.layout.first_inst(first_ebb).unwrap();
        Self {
            ebb: first_ebb,
            inst: first_inst,
        }
    }
}

impl Mutator for ReplaceInstWithIconst {
    fn name(&self) -> &'static str {
        "replace inst with iconst"
    }

    fn mutation_count(&self, func: &Function) -> Option<usize> {
        Some(inst_count(func))
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, MutationKind)> {
        if let Some((_prev_ebb, prev_inst)) =
            next_inst_ret_prev(&func, &mut self.ebb, &mut self.inst)
        {
            let results = func.dfg.inst_results(prev_inst);
            if results.len() == 1 {
                let ty = func.dfg.value_type(results[0]);
                func.dfg.replace(prev_inst).iconst(ty, 0);
                Some((
                    func,
                    format!("Replace inst {} with iconst.{}", prev_inst, ty),
                    MutationKind::Changed,
                ))
            } else {
                Some((func, format!(""), MutationKind::Changed))
            }
        } else {
            None
        }
    }
}

/// Try to replace instructions with `trap`.
struct ReplaceInstWithTrap {
    ebb: Ebb,
    inst: Inst,
}

impl ReplaceInstWithTrap {
    fn new(func: &Function) -> Self {
        let first_ebb = func.layout.entry_block().unwrap();
        let first_inst = func.layout.first_inst(first_ebb).unwrap();
        Self {
            ebb: first_ebb,
            inst: first_inst,
        }
    }
}

impl Mutator for ReplaceInstWithTrap {
    fn name(&self) -> &'static str {
        "replace inst with trap"
    }

    fn mutation_count(&self, func: &Function) -> Option<usize> {
        Some(inst_count(func))
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, MutationKind)> {
        if let Some((_prev_ebb, prev_inst)) =
            next_inst_ret_prev(&func, &mut self.ebb, &mut self.inst)
        {
            func.dfg.replace(prev_inst).trap(TrapCode::User(0));
            Some((
                func,
                format!("Replace inst {} with trap", prev_inst),
                MutationKind::Changed,
            ))
        } else {
            None
        }
    }
}

/// Try to remove an ebb.
struct RemoveEbb {
    ebb: Ebb,
}

impl RemoveEbb {
    fn new(func: &Function) -> Self {
        Self {
            ebb: func.layout.entry_block().unwrap(),
        }
    }
}

impl Mutator for RemoveEbb {
    fn name(&self) -> &'static str {
        "remove ebb"
    }

    fn mutation_count(&self, func: &Function) -> Option<usize> {
        Some(ebb_count(func))
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, MutationKind)> {
        if let Some(next_ebb) = func.layout.next_ebb(self.ebb) {
            self.ebb = next_ebb;
            while let Some(inst) = func.layout.last_inst(self.ebb) {
                func.layout.remove_inst(inst);
            }
            func.layout.remove_ebb(self.ebb);
            Some((
                func,
                format!("Remove ebb {}", next_ebb),
                MutationKind::Shrinked,
            ))
        } else {
            None
        }
    }
}

/// Try to remove unused entities.
struct RemoveUnusedEntities {
    kind: u32,
}

impl RemoveUnusedEntities {
    fn new() -> Self {
        Self { kind: 0 }
    }
}

impl Mutator for RemoveUnusedEntities {
    fn name(&self) -> &'static str {
        "remove unused entities"
    }

    fn mutation_count(&self, _func: &Function) -> Option<usize> {
        Some(4)
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, MutationKind)> {
        let name = match self.kind {
            0 => {
                let mut ext_func_usage_map = HashMap::new();
                for ebb in func.layout.ebbs() {
                    for inst in func.layout.ebb_insts(ebb) {
                        match func.dfg[inst] {
                            // Add new cases when there are new instruction formats taking a `FuncRef`.
                            InstructionData::Call { func_ref, .. }
                            | InstructionData::FuncAddr { func_ref, .. } => {
                                ext_func_usage_map
                                    .entry(func_ref)
                                    .or_insert_with(Vec::new)
                                    .push(inst);
                            }
                            _ => {}
                        }
                    }
                }

                let mut ext_funcs = PrimaryMap::new();

                for (func_ref, ext_func_data) in func.dfg.ext_funcs.clone().into_iter() {
                    if let Some(func_ref_usage) = ext_func_usage_map.get(&func_ref) {
                        let new_func_ref = ext_funcs.push(ext_func_data.clone());
                        for &inst in func_ref_usage {
                            match func.dfg[inst] {
                                // Keep in sync with the above match.
                                InstructionData::Call {
                                    ref mut func_ref, ..
                                }
                                | InstructionData::FuncAddr {
                                    ref mut func_ref, ..
                                } => {
                                    *func_ref = new_func_ref;
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }

                func.dfg.ext_funcs = ext_funcs;

                "Remove unused ext funcs"
            }
            1 => {
                #[derive(Copy, Clone)]
                enum SigRefUser {
                    Instruction(Inst),
                    ExtFunc(FuncRef),
                }

                let mut signatures_usage_map = HashMap::new();
                for ebb in func.layout.ebbs() {
                    for inst in func.layout.ebb_insts(ebb) {
                        match func.dfg[inst] {
                            // Add new cases when there are new instruction formats taking a `SigRef`.
                            InstructionData::CallIndirect { sig_ref, .. } => {
                                signatures_usage_map
                                    .entry(sig_ref)
                                    .or_insert_with(Vec::new)
                                    .push(SigRefUser::Instruction(inst));
                            }
                            _ => {}
                        }
                    }
                }
                for (func_ref, ext_func_data) in func.dfg.ext_funcs.iter() {
                    signatures_usage_map
                        .entry(ext_func_data.signature)
                        .or_insert_with(Vec::new)
                        .push(SigRefUser::ExtFunc(func_ref));
                }

                let mut signatures = PrimaryMap::new();

                for (sig_ref, sig_data) in func.dfg.signatures.clone().into_iter() {
                    if let Some(sig_ref_usage) = signatures_usage_map.get(&sig_ref) {
                        let new_sig_ref = signatures.push(sig_data.clone());
                        for &sig_ref_user in sig_ref_usage {
                            match sig_ref_user {
                                SigRefUser::Instruction(inst) => match func.dfg[inst] {
                                    // Keep in sync with the above match.
                                    InstructionData::CallIndirect {
                                        ref mut sig_ref, ..
                                    } => {
                                        *sig_ref = new_sig_ref;
                                    }
                                    _ => unreachable!(),
                                },
                                SigRefUser::ExtFunc(func_ref) => {
                                    func.dfg.ext_funcs[func_ref].signature = new_sig_ref;
                                }
                            }
                        }
                    }
                }

                func.dfg.signatures = signatures;

                "Remove unused signatures"
            }
            2 => {
                let mut stack_slot_usage_map = HashMap::new();
                for ebb in func.layout.ebbs() {
                    for inst in func.layout.ebb_insts(ebb) {
                        match func.dfg[inst] {
                            // Add new cases when there are new instruction formats taking a `StackSlot`.
                            InstructionData::StackLoad { stack_slot, .. }
                            | InstructionData::StackStore { stack_slot, .. } => {
                                stack_slot_usage_map
                                    .entry(stack_slot)
                                    .or_insert_with(Vec::new)
                                    .push(inst);
                            }

                            InstructionData::RegSpill { dst, .. } => {
                                stack_slot_usage_map
                                    .entry(dst)
                                    .or_insert_with(Vec::new)
                                    .push(inst);
                            }
                            InstructionData::RegFill { src, .. } => {
                                stack_slot_usage_map
                                    .entry(src)
                                    .or_insert_with(Vec::new)
                                    .push(inst);
                            }
                            _ => {}
                        }
                    }
                }

                let mut stack_slots = StackSlots::new();

                for (stack_slot, stack_slot_data) in func.stack_slots.clone().iter() {
                    if let Some(stack_slot_usage) = stack_slot_usage_map.get(&stack_slot) {
                        let new_stack_slot = stack_slots.push(stack_slot_data.clone());
                        for &inst in stack_slot_usage {
                            match &mut func.dfg[inst] {
                                // Keep in sync with the above match.
                                InstructionData::StackLoad { stack_slot, .. }
                                | InstructionData::StackStore { stack_slot, .. } => {
                                    *stack_slot = new_stack_slot;
                                }
                                InstructionData::RegSpill { dst, .. } => {
                                    *dst = new_stack_slot;
                                }
                                InstructionData::RegFill { src, .. } => {
                                    *src = new_stack_slot;
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }

                func.stack_slots = stack_slots;

                "Remove unused stack slots"
            }
            3 => {
                let mut global_value_usage_map = HashMap::new();
                for ebb in func.layout.ebbs() {
                    for inst in func.layout.ebb_insts(ebb) {
                        match func.dfg[inst] {
                            // Add new cases when there are new instruction formats taking a `GlobalValue`.
                            InstructionData::UnaryGlobalValue { global_value, .. } => {
                                global_value_usage_map
                                    .entry(global_value)
                                    .or_insert_with(Vec::new)
                                    .push(inst);
                            }
                            _ => {}
                        }
                    }
                }

                for (_global_value, global_value_data) in func.global_values.iter() {
                    match *global_value_data {
                        GlobalValueData::VMContext | GlobalValueData::Symbol { .. } => {}
                        // These can create cyclic references, which cause complications. Just skip
                        // the global value removal for now.
                        // FIXME Handle them in a better way.
                        GlobalValueData::Load { base: _, .. }
                        | GlobalValueData::IAddImm { base: _, .. } => return None,
                    }
                }

                let mut global_values = PrimaryMap::new();

                for (global_value, global_value_data) in func.global_values.clone().into_iter() {
                    if let Some(global_value_usage) = global_value_usage_map.get(&global_value) {
                        let new_global_value = global_values.push(global_value_data.clone());
                        for &inst in global_value_usage {
                            match &mut func.dfg[inst] {
                                // Keep in sync with the above match.
                                InstructionData::UnaryGlobalValue { global_value, .. } => {
                                    *global_value = new_global_value;
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }

                func.global_values = global_values;

                "Remove unused global values"
            }
            _ => return None,
        };
        self.kind += 1;
        Some((func, name.to_owned(), MutationKind::Changed))
    }
}

fn next_inst_ret_prev(func: &Function, ebb: &mut Ebb, inst: &mut Inst) -> Option<(Ebb, Inst)> {
    let prev = (*ebb, *inst);
    if let Some(next_inst) = func.layout.next_inst(*inst) {
        *inst = next_inst;
        return Some(prev);
    } else if let Some(next_ebb) = func.layout.next_ebb(*ebb) {
        *ebb = next_ebb;
        *inst = func.layout.first_inst(*ebb).expect("no inst");
        return Some(prev);
    } else {
        return None;
    }
}

fn ebb_count(func: &Function) -> usize {
    func.layout.ebbs().count()
}

fn inst_count(func: &Function) -> usize {
    func.layout
        .ebbs()
        .map(|ebb| func.layout.ebb_insts(ebb).count())
        .sum()
}

fn resolve_aliases(func: &mut Function) {
    for ebb in func.layout.ebbs() {
        for inst in func.layout.ebb_insts(ebb) {
            func.dfg.resolve_aliases_in_arguments(inst);
        }
    }
}

fn reduce(
    isa: &dyn TargetIsa,
    mut func: Function,
    verbose: bool,
) -> Result<(Function, String), String> {
    let mut ccc = CrashCheckContext::new(isa);

    match ccc.check_for_crash(&func) {
        CheckResult::Succeed => {
            return Err(format!(
                "Given function compiled successfully or gave an verifier error."
            ));
        }
        CheckResult::Crash(_) => {}
    }

    resolve_aliases(&mut func);

    for pass_idx in 0..100 {
        let mut should_keep_reducing = false;
        let mut phase = 0;

        loop {
            let mut mutator = match phase {
                0 => Box::new(RemoveInst::new(&func)) as Box<dyn Mutator>,
                1 => Box::new(ReplaceInstWithIconst::new(&func)) as Box<dyn Mutator>,
                2 => Box::new(ReplaceInstWithTrap::new(&func)) as Box<dyn Mutator>,
                3 => Box::new(RemoveEbb::new(&func)) as Box<dyn Mutator>,
                4 => Box::new(RemoveUnusedEntities::new()) as Box<dyn Mutator>,
                _ => break,
            };

            func = mutator.reduce(
                &mut ccc,
                func,
                format!("pass {}", pass_idx),
                verbose,
                &mut should_keep_reducing,
            );

            phase += 1;
        }

        if !should_keep_reducing {
            // No new shrinking opportunities have been found this pass. This means none will ever
            // be found. Skip the rest of the passes over the function.
            break;
        }
    }

    let crash_msg = match ccc.check_for_crash(&func) {
        CheckResult::Succeed => unreachable!("Used to crash, but doesn't anymore???"),
        CheckResult::Crash(crash_msg) => crash_msg,
    };

    Ok((func, crash_msg))
}

struct CrashCheckContext<'a> {
    /// Cached `Context`, to prevent repeated allocation.
    context: Context,

    /// The target isa to compile for.
    isa: &'a dyn TargetIsa,
}

fn get_panic_string(panic: Box<dyn std::any::Any>) -> String {
    let panic = match panic.downcast::<&'static str>() {
        Ok(panic_msg) => panic_msg.to_owned(),
        Err(panic) => panic,
    };
    match panic.downcast::<String>() {
        Ok(panic_msg) => *panic_msg,
        Err(_) => "Box<Any>".to_owned(),
    }
}

enum CheckResult {
    /// The function compiled fine, or the verifier noticed an error.
    Succeed,

    /// The compilation of the function panicked.
    Crash(String),
}

impl<'a> CrashCheckContext<'a> {
    fn new(isa: &'a dyn TargetIsa) -> Self {
        CrashCheckContext {
            context: Context::new(),
            isa,
        }
    }

    #[cfg_attr(test, allow(unreachable_code))]
    fn check_for_crash(&mut self, func: &Function) -> CheckResult {
        self.context.clear();
        self.context.func = func.clone();

        use std::io::Write;
        std::io::stdout().flush().unwrap(); // Flush stdout to sync with panic messages on stderr

        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cranelift_codegen::verifier::verify_function(&func, self.isa).err()
        })) {
            Ok(Some(_)) => return CheckResult::Succeed,
            Ok(None) => {}
            // The verifier panicked. compiling it will probably give the same panic.
            // We treat it as succeeding to make it possible to reduce for the actual error.
            // FIXME prevent verifier panic on removing ebb1
            Err(_) => return CheckResult::Succeed,
        }

        #[cfg(test)]
        {
            // For testing purposes we emulate a panic caused by the existence of
            // a `call` instruction.
            let contains_call = func.layout.ebbs().any(|ebb| {
                func.layout.ebb_insts(ebb).any(|inst| match func.dfg[inst] {
                    InstructionData::Call { .. } => true,
                    _ => false,
                })
            });
            if contains_call {
                return CheckResult::Crash("test crash".to_string());
            } else {
                return CheckResult::Succeed;
            }
        }

        let old_panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // silence panics

        let res = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut relocs = PrintRelocs::new(false);
            let mut traps = PrintTraps::new(false);
            let mut stackmaps = PrintStackmaps::new(false);
            let mut mem = vec![];

            let _ = self.context.compile_and_emit(
                self.isa,
                &mut mem,
                &mut relocs,
                &mut traps,
                &mut stackmaps,
            );
        })) {
            Ok(()) => CheckResult::Succeed,
            Err(err) => CheckResult::Crash(get_panic_string(err)),
        };

        std::panic::set_hook(old_panic_hook);

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_reader::ParseOptions;

    #[test]
    fn test_reduce() {
        const TEST: &'static str = include_str!("./bugpoint_test.clif");

        let test_file = parse_test(TEST, ParseOptions::default()).unwrap();

        // If we have an isa from the command-line, use that. Otherwise if the
        // file contains a unique isa, use that.
        let isa = test_file.isa_spec.unique_isa().expect("Unknown isa");

        for (func, _) in test_file.functions {
            let (func, crash_msg) = reduce(isa, func, false).expect("Couldn't reduce test case");

            assert_eq!(crash_msg, "test crash");

            assert_eq!(
                format!("{}", func),
                "function u0:0(i64, i64, i64) system_v {
    sig0 = (i64, i64, i16, i64, i64, i64, i64, i64) system_v
    fn0 = u0:95 sig0

ebb0(v0: i64, v1: i64, v2: i64):
    v113 -> v1
    v124 -> v1
    v136 -> v1
    v148 -> v1
    v160 -> v1
    v185 -> v1
    v222 -> v1
    v237 -> v1
    v241 -> v1
    v256 -> v1
    v262 -> v1
    v105 = iconst.i64 0
    trap user0

ebb99(v804: i64, v1035: i64, v1037: i64, v1039: i64, v1044: i64, v1052: i16, v1057: i64):
    v817 -> v1035
    v830 -> v1037
    v844 -> v1039
    v857 -> v1039
    v939 -> v1039
    v1042 -> v1039
    v1050 -> v1039
    v908 -> v1044
    v917 -> v1044
    v921 -> v1044
    v1043 -> v1044
    v960 -> v1052
    v990 -> v1052
    v1051 -> v1052
    v1055 -> v1052
    v963 -> v1057
    v1056 -> v1057
    v1060 -> v1057
    trap user0

ebb101:
    v829 = iconst.i64 0
    v935 -> v829
    v962 -> v829
    v992 -> v829
    v1036 -> v829
    v1049 -> v829
    trap user0

ebb102:
    v842 = iconst.i64 0
    v976 -> v842
    v989 -> v842
    v1038 -> v842
    v1061 -> v842
    trap user0

ebb105:
    v883 = iconst.i64 0
    v934 -> v883
    v961 -> v883
    v991 -> v883
    v1005 -> v883
    v1048 -> v883
    trap user0

ebb114:
    v951 = iconst.i64 0
    v988 -> v951
    trap user0

ebb117:
    v987 = iconst.i64 0
    call fn0(v0, v105, v1052, v883, v829, v987, v951, v842)
    trap user0
}
"
            );
        }
    }
}
