//! Generator for exception-handling fuzz test cases.
//!
//! Generates Wasm modules that exercise `throw`/`try_table`/`catch` in
//! various combinations: multiple tags with different signatures, nested
//! handler scopes, throws at various call depths, catch vs catch_all, and
//! multiple catch clauses per try_table.

use mutatis::{Candidates, Context, DefaultMutate, Generate, Mutate, Result as MutResult};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use wasm_encoder::{
    BlockType, CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection,
    ImportSection, Instruction, Module, TagKind, TagSection, TagType, TypeSection, ValType,
};

/// Max number of tags.
pub const NUM_TAGS_MAX: u32 = 8;
/// Max call-chain depth (number of functions between run and thrower).
pub const CALL_DEPTH_MAX: u32 = 6;
/// Max number of scenarios (throw+catch combos) per test case.
pub const MAX_SCENARIOS: usize = 16;
/// Maximum params per tag signature.
pub const MAX_TAG_PARAMS: usize = 4;
/// Maximum number of decoy catches.
pub const MAX_DECOY_CATCHES: usize = 4;

/// Limits controlling the structure of a generated Wasm module.
#[derive(Debug, Default, Clone, Serialize, Deserialize, Mutate)]
pub struct ExceptionOpsLimits {
    /// Number of distinct tags to define.
    pub(crate) num_tags: u32,
    /// Depth of the call chain (number of intermediate functions).
    pub(crate) call_depth: u32,
}

impl ExceptionOpsLimits {
    pub(crate) fn fixup(&mut self) {
        self.num_tags = self.num_tags.clamp(1, NUM_TAGS_MAX);
        self.call_depth = self.call_depth.clamp(1, CALL_DEPTH_MAX);
    }
}

/// A tag signature.
#[derive(Debug, Clone, Serialize, Deserialize, Mutate)]
pub struct TagSig {
    pub(crate) params: Vec<SimpleValType>,
}

/// Subset of ValType that we generate for tag params.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs, reason = "self-describing")]
pub enum SimpleValType {
    I32,
    I64,
    F32,
    F64,
}

impl SimpleValType {
    fn to_val_type(self) -> ValType {
        match self {
            Self::I32 => ValType::I32,
            Self::I64 => ValType::I64,
            Self::F32 => ValType::F32,
            Self::F64 => ValType::F64,
        }
    }

    /// A deterministic "interesting" constant for the given type and index,
    /// used as the thrown payload so the oracle can verify the catch.
    fn test_value(self, idx: u32) -> Instruction<'static> {
        match self {
            Self::I32 => Instruction::I32Const(0x1000_i32.wrapping_add(idx.cast_signed())),
            Self::I64 => Instruction::I64Const(0x2000_i64.wrapping_add(i64::from(idx))),
            Self::F32 => Instruction::F32Const(wasm_encoder::Ieee32::new(0x4000_0000 + idx)),
            Self::F64 => Instruction::F64Const(wasm_encoder::Ieee64::new(
                0x4000_0000_0000_0000 + u64::from(idx),
            )),
        }
    }
}

/// Which kind of catch clause to use for the handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatchKind {
    /// `(catch $tag $label)`: specific tag, payload on branch.
    Catch,
    /// `(catch_all $label)`: any tag, no payload on branch.
    CatchAll,
}

/// One throw-and-catch scenario.
#[derive(Debug, Clone, Serialize, Deserialize, Mutate)]
pub struct Scenario {
    /// Index (into our tag list) of the tag to throw.
    pub(crate) throw_tag: u32,
    /// The function depth at which the throw happens (0 = run, call_depth = deepest).
    pub(crate) throw_depth: u32,
    /// The function depth at which the handler lives. Must be <= throw_depth.
    pub(crate) catch_depth: u32,
    /// What kind of catch clause to use.
    pub(crate) catch_kind: CatchKind,
    /// Extra catch clauses (tag indices) to place *before* the real one in the
    /// try_table, exercising the "skip non-matching" path. These must be
    /// indices of tags different from `throw_tag`.
    pub(crate) decoy_catches: Vec<u32>,
    /// Extra catch clauses (tag indices) to place *after* the real one in the
    /// try_table, exercising the "clauses past the match" path. These must be
    /// indices of tags different from `throw_tag`.
    pub(crate) decoy_catches_after: Vec<u32>,
}

/// A description of a Wasm module that exercises exception throw/catch.
#[derive(Debug, Default, Clone, Serialize, Deserialize, Mutate)]
pub struct ExceptionOps {
    pub(crate) limits: ExceptionOpsLimits,
    pub(crate) tag_sigs: Vec<TagSig>,
    pub(crate) scenarios: Vec<Scenario>,
}

impl ExceptionOps {
    /// Encode as a Wasm module.
    pub fn to_wasm_binary(&mut self) -> Vec<u8> {
        self.fixup();
        self.encode()
    }

    /// Fix up the test case to ensure all indices and structures are valid.
    pub fn fixup(&mut self) {
        self.limits.fixup();
        let num_tags = usize::try_from(self.limits.num_tags).unwrap();

        // Ensure we have exactly num_tags tag signatures.
        while self.tag_sigs.len() < num_tags {
            // Default: single i32 param.
            self.tag_sigs.push(TagSig {
                params: vec![SimpleValType::I32],
            });
        }
        self.tag_sigs.truncate(num_tags);

        // Clamp param counts.
        for sig in &mut self.tag_sigs {
            sig.params.truncate(MAX_TAG_PARAMS);
            if sig.params.is_empty() {
                sig.params.push(SimpleValType::I32);
            }
        }

        // Ensure at least one scenario.
        if self.scenarios.is_empty() {
            self.scenarios.push(Scenario {
                throw_tag: 0,
                throw_depth: self.limits.call_depth,
                catch_depth: 0,
                catch_kind: CatchKind::Catch,
                decoy_catches: vec![],
                decoy_catches_after: vec![],
            });
        }
        self.scenarios.truncate(MAX_SCENARIOS);

        let num_tags = self.limits.num_tags;
        let depth = self.limits.call_depth;
        for s in &mut self.scenarios {
            s.throw_tag = s.throw_tag % num_tags;
            s.throw_depth = s.throw_depth.clamp(1, depth);
            s.catch_depth = s.catch_depth.clamp(0, s.throw_depth - 1);
            // Decoy catches: keep only tags different from the real one.
            s.decoy_catches.retain(|t| *t % num_tags != s.throw_tag);
            for t in &mut s.decoy_catches {
                *t = *t % num_tags;
            }
            s.decoy_catches.truncate(MAX_DECOY_CATCHES);
            s.decoy_catches_after
                .retain(|t| *t % num_tags != s.throw_tag);
            for t in &mut s.decoy_catches_after {
                *t = *t % num_tags;
            }
            s.decoy_catches_after.truncate(MAX_DECOY_CATCHES);
        }
    }

    /// Module layout:
    ///
    /// Types:
    ///   0..num_tags                       tag signatures: (params) -> ()
    ///   num_tags..2*num_tags              catch block types: () -> (params)
    ///   2*num_tags                        () -> (i32)  (run & relay functions)
    ///   2*num_tags + 1                    (i32, i32) -> ()  (check_i32)
    ///   2*num_tags + 2                    () -> ()  (thrower functions)
    ///
    /// Tags:  0..num_tags (using type indices 0..num_tags)
    /// Imports:  0: "check_i32"
    /// Functions:  per-scenario call chains + "run"
    /// Exports:  "run"
    fn encode(&self) -> Vec<u8> {
        let num_tags = self.limits.num_tags;
        let call_depth = self.limits.call_depth;
        let num_scenarios = u32::try_from(self.scenarios.len()).unwrap();

        let mut module = Module::new();

        let mut types = TypeSection::new();

        // Type indices 0..num_tags: tag signatures (params -> [])
        for sig in &self.tag_sigs {
            let params: Vec<ValType> = sig.params.iter().map(|t| t.to_val_type()).collect();
            types.ty().function(params, vec![]);
        }

        // Type indices num_tags..2*num_tags: catch block types ([] -> params)
        // These are used as block types for the catch target blocks, since
        // `catch $tag $label` delivers the tag's param types at the label.
        let catch_block_type_base = num_tags;
        for sig in &self.tag_sigs {
            let results: Vec<ValType> = sig.params.iter().map(|t| t.to_val_type()).collect();
            types.ty().function(vec![], results);
        }

        // Utility function types
        let fn_type_void_to_i32 = types.len();
        types.ty().function(vec![], vec![ValType::I32]);

        let fn_type_check = types.len();
        types
            .ty()
            .function(vec![ValType::I32, ValType::I32], vec![]);

        let fn_type_void_to_void = types.len();
        types.ty().function(vec![], vec![]);

        let mut tags = TagSection::new();
        for i in 0..num_tags {
            tags.tag(TagType {
                kind: TagKind::Exception,
                func_type_idx: i,
            });
        }

        let mut imports = ImportSection::new();
        let check_func_idx: u32 = 0;
        imports.import("", "check_i32", EntityType::Function(fn_type_check));
        let import_count: u32 = imports.len();

        // For each scenario: (call_depth + 1) functions at depths 0..=call_depth.
        //   - depth == catch_depth: handler function () -> (i32)
        //   - depth == throw_depth: thrower function () -> ()
        //   - other depths < throw_depth: relay function () -> (i32)
        //   - other depths > throw_depth: unreachable () -> ()
        //
        // Plus one "run" function that calls each scenario's depth-0 function.

        let funcs_per_scenario = call_depth + 1;
        let run_defined_idx = num_scenarios * funcs_per_scenario;
        let run_func_idx = import_count + run_defined_idx;

        let mut functions = FunctionSection::new();
        let mut code = CodeSection::new();

        for (si, scenario) in self.scenarios.iter().enumerate() {
            let scenario_base = import_count + u32::try_from(si).unwrap() * funcs_per_scenario;
            let tag_sig = &self.tag_sigs[usize::try_from(scenario.throw_tag).unwrap()];

            for d in 0..=call_depth {
                if d == scenario.throw_depth {
                    // -- Thrower: pushes payload, throws tag --
                    functions.function(fn_type_void_to_void);
                    let mut f = Function::new(vec![]);
                    for (pi, param_ty) in tag_sig.params.iter().enumerate() {
                        f.instruction(&param_ty.test_value(
                            scenario.throw_tag * u32::try_from(MAX_TAG_PARAMS).unwrap()
                                + u32::try_from(pi).unwrap(),
                        ));
                    }
                    f.instruction(&Instruction::Throw(scenario.throw_tag));
                    f.instruction(&Instruction::End);
                    code.function(&f);
                } else if d == scenario.catch_depth {
                    // -- Handler: try_table with catch clauses --
                    functions.function(fn_type_void_to_i32);
                    let mut f = Function::new(vec![]);

                    // All decoys (before + after the real catch) share the same
                    // block structure; only the catch-clause ordering differs.
                    let all_decoys: Vec<u32> = scenario
                        .decoy_catches
                        .iter()
                        .chain(scenario.decoy_catches_after.iter())
                        .copied()
                        .collect();
                    let num_decoys = all_decoys.len();
                    let num_before = scenario.decoy_catches.len();

                    // Block nesting (outermost to innermost):
                    //   block $result (result i32)
                    //     block $catch (result <tag_params> or empty)
                    //       block $decoy_0 (result <decoy params>)
                    //         ...
                    //         block $decoy_{n-1} (result <decoy params>)
                    //           try_table ...
                    //
                    // Catch clause labels are relative to the enclosing blocks,
                    // not counting the try_table itself. So from the catch:
                    //   label 0 = $decoy_{n-1} (innermost block)
                    //   label num_decoys-1 = $decoy_0
                    //   label num_decoys = $catch
                    //   label num_decoys+1 = $result
                    //
                    // br instructions inside the try_table body do count the
                    // try_table, so they need +1 compared to catch labels.

                    let catch_label = u32::try_from(num_decoys).unwrap();
                    let result_label = u32::try_from(num_decoys + 1).unwrap();

                    // For br inside try_table body, add 1 for the try_table scope
                    let br_result_label = result_label + 1;

                    // Build catch clauses: before-decoys, real catch, after-decoys
                    let mut catches: Vec<wasm_encoder::Catch> = Vec::new();
                    for (di, &decoy_tag) in scenario.decoy_catches.iter().enumerate() {
                        let decoy_label = u32::try_from(num_decoys - 1 - di).unwrap();
                        catches.push(wasm_encoder::Catch::One {
                            tag: decoy_tag,
                            label: decoy_label,
                        });
                    }
                    match scenario.catch_kind {
                        CatchKind::Catch => {
                            catches.push(wasm_encoder::Catch::One {
                                tag: scenario.throw_tag,
                                label: catch_label,
                            });
                        }
                        CatchKind::CatchAll => {
                            catches.push(wasm_encoder::Catch::All { label: catch_label });
                        }
                    }
                    for (i, &decoy_tag) in scenario.decoy_catches_after.iter().enumerate() {
                        let di = num_before + i;
                        let decoy_label = u32::try_from(num_decoys - 1 - di).unwrap();
                        catches.push(wasm_encoder::Catch::One {
                            tag: decoy_tag,
                            label: decoy_label,
                        });
                    }

                    // Emit blocks (outermost first)
                    // block $result (result i32)
                    f.instruction(&Instruction::Block(BlockType::Result(ValType::I32)));

                    // block $catch
                    match scenario.catch_kind {
                        CatchKind::Catch => {
                            let bt =
                                BlockType::FunctionType(catch_block_type_base + scenario.throw_tag);
                            f.instruction(&Instruction::Block(bt));
                        }
                        CatchKind::CatchAll => {
                            f.instruction(&Instruction::Block(BlockType::Empty));
                        }
                    }

                    // Decoy blocks (decoy_0 outermost, decoy_{n-1} innermost)
                    for &decoy_tag in &all_decoys {
                        let bt = BlockType::FunctionType(catch_block_type_base + decoy_tag);
                        f.instruction(&Instruction::Block(bt));
                    }

                    // try_table (empty body type -- all exits via branches)
                    f.instruction(&Instruction::TryTable(
                        BlockType::Empty,
                        Cow::Borrowed(&catches),
                    ));

                    // Body: call next deeper function
                    let next_func = scenario_base + d + 1;
                    f.instruction(&Instruction::Call(next_func));

                    // Normal completion (no exception): push 0, branch to $result
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::Br(br_result_label));

                    f.instruction(&Instruction::End); // end try_table
                    // Normal flow always exits via br above; make fall-through unreachable
                    f.instruction(&Instruction::Unreachable);

                    // End decoy blocks (innermost first) — drop all payload, push -1
                    // After ending $decoy_{di}, we're inside $decoy_{di-1}
                    // (or $catch if di==0). Depth to $result = di + 1.
                    for di in (0..num_decoys).rev() {
                        f.instruction(&Instruction::End); // end block $decoy_{di}
                        // Drop the caught payload values
                        let decoy_tag = all_decoys[di];
                        let decoy_sig = &self.tag_sigs[usize::try_from(decoy_tag).unwrap()];
                        for _ in &decoy_sig.params {
                            f.instruction(&Instruction::Drop);
                        }
                        // Wrong tag caught -- return -1
                        f.instruction(&Instruction::I32Const(-1));
                        let depth_to_result = u32::try_from(di).unwrap() + 1;
                        f.instruction(&Instruction::Br(depth_to_result));
                    }

                    // End $catch block -- verify payload and return 1
                    f.instruction(&Instruction::End); // end block $catch
                    match scenario.catch_kind {
                        CatchKind::Catch => {
                            // All tag param values are on the stack.
                            // Verify the first i32 value if present, drop the rest.
                            let nparams = tag_sig.params.len();
                            // Drop all values from top (last param) down to first
                            for _ in 1..nparams {
                                f.instruction(&Instruction::Drop);
                            }
                            // First param is now on top
                            if tag_sig.params[0] == SimpleValType::I32 {
                                let idx =
                                    scenario.throw_tag * u32::try_from(MAX_TAG_PARAMS).unwrap();
                                let expected = 0x1000_i32.wrapping_add(i32::try_from(idx).unwrap());
                                f.instruction(&Instruction::I32Const(expected));
                                f.instruction(&Instruction::Call(check_func_idx));
                            } else {
                                f.instruction(&Instruction::Drop);
                            }
                            f.instruction(&Instruction::I32Const(1));
                        }
                        CatchKind::CatchAll => {
                            f.instruction(&Instruction::I32Const(1));
                        }
                    }

                    f.instruction(&Instruction::End); // end block $result
                    f.instruction(&Instruction::End); // end function
                    code.function(&f);
                } else if d < scenario.throw_depth && d < scenario.catch_depth {
                    // -- Relay above handler: calls next deeper, returns i32 --
                    functions.function(fn_type_void_to_i32);
                    let mut f = Function::new(vec![]);
                    let next_func = scenario_base + d + 1;
                    f.instruction(&Instruction::Call(next_func));
                    f.instruction(&Instruction::End);
                    code.function(&f);
                } else if d < scenario.throw_depth {
                    // -- Relay between handler and thrower: void --
                    // These functions will never return normally (thrower always throws),
                    // but the validator needs the types to be correct.
                    functions.function(fn_type_void_to_void);
                    let mut f = Function::new(vec![]);
                    let next_func = scenario_base + d + 1;
                    f.instruction(&Instruction::Call(next_func));
                    f.instruction(&Instruction::End);
                    code.function(&f);
                } else {
                    // -- Dead code beyond throw depth --
                    functions.function(fn_type_void_to_void);
                    let mut f = Function::new(vec![]);
                    f.instruction(&Instruction::Unreachable);
                    f.instruction(&Instruction::End);
                    code.function(&f);
                }
            }
        }

        // -- "run" function: calls each scenario entry, sums results --
        functions.function(fn_type_void_to_i32);
        {
            let mut f = Function::new(vec![(1, ValType::I32)]);
            for si in 0..num_scenarios {
                let scenario_entry = import_count + si * funcs_per_scenario;
                f.instruction(&Instruction::Call(scenario_entry));
                f.instruction(&Instruction::LocalGet(0));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::LocalSet(0));
            }
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::End);
            code.function(&f);
        }

        let mut exports = ExportSection::new();
        exports.export("run", ExportKind::Func, run_func_idx);

        module
            .section(&types)
            .section(&imports)
            .section(&functions)
            .section(&tags)
            .section(&exports)
            .section(&code);

        module.finish()
    }

    /// Pop the last scenario. Returns true if one was removed.
    pub fn pop(&mut self) -> bool {
        self.scenarios.pop().is_some()
    }

    /// Number of scenarios; expected return value from "run" when all
    /// catches succeed.
    pub fn expected_result(&mut self) -> i32 {
        self.fixup();
        i32::try_from(self.scenarios.len()).unwrap()
    }
}

/// Mutator for unit-variant enums ([`SimpleValType`] and [`CatchKind`]),
/// which need manual impls because `#[derive(Mutate)]` doesn't switch
/// between variants.
#[derive(Debug, Default)]
pub struct EnumMutator;

impl Mutate<SimpleValType> for EnumMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, value: &mut SimpleValType) -> MutResult<()> {
        c.mutation(|ctx| {
            let choices = [
                SimpleValType::I32,
                SimpleValType::I64,
                SimpleValType::F32,
                SimpleValType::F64,
            ];
            *value = *ctx.rng().choose(&choices).unwrap();
            Ok(())
        })?;
        Ok(())
    }
}

impl Generate<SimpleValType> for EnumMutator {
    fn generate(&mut self, ctx: &mut Context) -> MutResult<SimpleValType> {
        let choices = [
            SimpleValType::I32,
            SimpleValType::I64,
            SimpleValType::F32,
            SimpleValType::F64,
        ];
        Ok(*ctx.rng().choose(&choices).unwrap())
    }
}

impl DefaultMutate for SimpleValType {
    type DefaultMutate = EnumMutator;
}

impl Mutate<CatchKind> for EnumMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, value: &mut CatchKind) -> MutResult<()> {
        c.mutation(|ctx| {
            let choices = [CatchKind::Catch, CatchKind::CatchAll];
            *value = *ctx.rng().choose(&choices).unwrap();
            Ok(())
        })?;
        Ok(())
    }
}

impl Generate<CatchKind> for EnumMutator {
    fn generate(&mut self, ctx: &mut Context) -> MutResult<CatchKind> {
        let choices = [CatchKind::Catch, CatchKind::CatchAll];
        Ok(*ctx.rng().choose(&choices).unwrap())
    }
}

impl DefaultMutate for CatchKind {
    type DefaultMutate = EnumMutator;
}

impl Generate<TagSig> for TagSigMutator {
    fn generate(&mut self, ctx: &mut Context) -> MutResult<TagSig> {
        let count = ctx.rng().gen_index(MAX_TAG_PARAMS).unwrap_or(0) + 1;
        let params = (0..count)
            .map(|_| EnumMutator.generate(ctx))
            .collect::<MutResult<Vec<_>>>()?;
        Ok(TagSig { params })
    }
}

impl Generate<Scenario> for ScenarioMutator {
    fn generate(&mut self, ctx: &mut Context) -> MutResult<Scenario> {
        let mut m = mutatis::mutators::u32();
        Ok(Scenario {
            throw_tag: mutatis::Generate::<u32>::generate(&mut m, ctx)?,
            throw_depth: mutatis::Generate::<u32>::generate(&mut m, ctx)?,
            catch_depth: mutatis::Generate::<u32>::generate(&mut m, ctx)?,
            catch_kind: EnumMutator.generate(ctx)?,
            decoy_catches: vec![],
            decoy_catches_after: vec![],
        })
    }
}

impl Generate<ExceptionOps> for ExceptionOpsMutator {
    fn generate(&mut self, _ctx: &mut Context) -> MutResult<ExceptionOps> {
        let mut ops = ExceptionOps::default();
        let mut session = mutatis::Session::new();
        for _ in 0..32 {
            session.mutate(&mut ops)?;
        }
        Ok(ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmparser::WasmFeatures;

    #[test]
    fn always_produces_valid_wasm() {
        mutatis::check::Check::new()
            .iters(200)
            .run(|ops: &ExceptionOps| {
                let mut ops = ops.clone();
                let wasm = ops.to_wasm_binary();
                let features = WasmFeatures::EXCEPTIONS
                    | WasmFeatures::GC_TYPES
                    | WasmFeatures::REFERENCE_TYPES
                    | WasmFeatures::MULTI_VALUE
                    | WasmFeatures::FLOATS
                    | WasmFeatures::SIMD;
                let mut validator = wasmparser::Validator::new_with_features(features);
                validator
                    .validate_all(&wasm)
                    .map(|_| ())
                    .map_err(|e| format!("{e}\n{ops:#?}"))
            })
            .unwrap();
    }
}
