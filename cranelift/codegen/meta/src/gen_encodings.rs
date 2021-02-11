//! Generate sources for instruction encoding.
//!
//! The tables and functions generated here support the `TargetISA::encode()` function which
//! determines if a given instruction is legal, and if so, its `Encoding` data which consists of a
//! *recipe* and some *encoding* bits.
//!
//! The `encode` function doesn't actually generate the binary machine bits. Each recipe has a
//! corresponding hand-written function to do that after registers are allocated.
//!
//! This is the information available to us:
//!
//! - The instruction to be encoded as an `InstructionData` reference.
//! - The controlling type variable.
//! - The data-flow graph giving us access to the types of all values involved. This is needed for
//! testing any secondary type variables.
//! - A `PredicateView` reference for the ISA-specific settings for evaluating ISA predicates.
//! - The currently active CPU mode is determined by the ISA.
//!
//! ## Level 1 table lookup
//!
//! The CPU mode provides the first table. The key is the instruction's controlling type variable.
//! If the instruction is not polymorphic, use `INVALID` for the type variable. The table values
//! are level 2 tables.
//!
//! ## Level 2 table lookup
//!
//! The level 2 table is keyed by the instruction's opcode. The table values are *encoding lists*.
//!
//! The two-level table lookup allows the level 2 tables to be much smaller with good locality.
//! Code in any given function usually only uses a few different types, so many of the level 2
//! tables will be cold.
//!
//! ## Encoding lists
//!
//! An encoding list is a non-empty sequence of list entries. Each entry has one of these forms:
//!
//! 1. Recipe + bits. Use this encoding if the recipe predicate is satisfied.
//! 2. Recipe + bits, final entry. Use this encoding if the recipe predicate is satisfied.
//!    Otherwise, stop with the default legalization code.
//! 3. Stop with legalization code.
//! 4. Predicate + skip count. Test predicate and skip N entries if it is false.
//! 5. Predicate + stop. Test predicate and stop with the default legalization code if it is false.
//!
//! The instruction predicate is also used to distinguish between polymorphic instructions with
//! different types for secondary type variables.

use std::collections::btree_map;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::TryFrom;
use std::iter::FromIterator;

use cranelift_codegen_shared::constant_hash::generate_table;
use cranelift_entity::EntityRef;

use crate::error;
use crate::srcgen::Formatter;

use crate::cdsl::cpu_modes::CpuMode;
use crate::cdsl::encodings::Encoding;
use crate::cdsl::instructions::{Instruction, InstructionPredicate, InstructionPredicateNumber};
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::recipes::{EncodingRecipe, OperandConstraint, Recipes, Register};
use crate::cdsl::regs::IsaRegs;
use crate::cdsl::settings::SettingPredicateNumber;
use crate::cdsl::types::ValueType;
use crate::cdsl::xform::TransformGroupIndex;

use crate::shared::Definitions as SharedDefinitions;

use crate::default_map::MapWithDefault;
use crate::unique_table::UniqueSeqTable;

/// Emit code for matching an instruction predicate against an `InstructionData` reference called
/// `inst`.
///
/// The generated code is an `if let` pattern match that falls through if the instruction has an
/// unexpected format. This should lead to a panic.
fn emit_instp(instp: &InstructionPredicate, has_func: bool, fmt: &mut Formatter) {
    if let Some(type_predicate) = instp.type_predicate("func") {
        fmt.line("let args = inst.arguments(&func.dfg.value_lists);");
        fmt.line(type_predicate);
        return;
    }

    let leaves = instp.collect_leaves();

    let mut has_type_check = false;
    let mut format_name = None;
    let mut field_names = HashSet::new();

    for leaf in leaves {
        if leaf.is_type_predicate() {
            has_type_check = true;
        } else {
            field_names.insert(leaf.format_destructuring_member_name());
            let leaf_format_name = leaf.format_name();
            match format_name {
                None => format_name = Some(leaf_format_name),
                Some(previous_format_name) => {
                    assert!(
                        previous_format_name == leaf_format_name,
                        "Format predicate can only operate on a single InstructionFormat; trying to use both {} and {}", previous_format_name, leaf_format_name
                    );
                }
            }
        }
    }

    let mut fields = Vec::from_iter(field_names);
    fields.sort();
    let fields = fields.join(", ");

    let format_name = format_name.expect("There should be a format name!");

    fmtln!(
        fmt,
        "if let crate::ir::InstructionData::{} {{ {}, .. }} = *inst {{",
        format_name,
        fields
    );
    fmt.indent(|fmt| {
        if has_type_check {
            // We could implement this.
            assert!(has_func, "recipe predicates can't check type variables.");
            fmt.line("let args = inst.arguments(&func.dfg.value_lists);");
        } else if has_func {
            // Silence dead argument.
            fmt.line("let _ = func;");
        }
        fmtln!(fmt, "return {};", instp.rust_predicate("func").unwrap());
    });
    fmtln!(fmt, "}");

    fmt.line("unreachable!();");
}

/// Emit private functions for checking recipe predicates as well as a static `RECIPE_PREDICATES`
/// array indexed by recipe number.
///
/// A recipe predicate is a combination of an ISA predicate and an instruction predicate. Many
/// recipes have identical predicates.
fn emit_recipe_predicates(isa: &TargetIsa, fmt: &mut Formatter) {
    let mut predicate_names = HashMap::new();

    fmt.comment(format!("{} recipe predicates.", isa.name));
    for recipe in isa.recipes.values() {
        let (isap, instp) = match (&recipe.isa_predicate, &recipe.inst_predicate) {
            (None, None) => continue,
            (isap, instp) if predicate_names.contains_key(&(isap, instp)) => continue,
            (isap, instp) => (isap, instp),
        };

        let func_name = format!("recipe_predicate_{}", recipe.name.to_lowercase());
        predicate_names.insert((isap, instp), func_name.clone());

        // Generate the predicate function.
        fmtln!(
            fmt,
            "fn {}({}: crate::settings::PredicateView, {}: &ir::InstructionData) -> bool {{",
            func_name,
            if isap.is_some() { "isap" } else { "_" },
            if instp.is_some() { "inst" } else { "_" }
        );
        fmt.indent(|fmt| {
            match (isap, instp) {
                (Some(isap), None) => {
                    fmtln!(fmt, "isap.test({})", isap);
                }
                (None, Some(instp)) => {
                    emit_instp(instp, /* has func */ false, fmt);
                }
                (Some(isap), Some(instp)) => {
                    fmtln!(fmt, "isap.test({}) &&", isap);
                    emit_instp(instp, /* has func */ false, fmt);
                }
                _ => panic!("skipped above"),
            }
        });
        fmtln!(fmt, "}");
    }
    fmt.empty_line();

    // Generate the static table.
    fmt.doc_comment(format!(
        r#"{} recipe predicate table.

        One entry per recipe, set to Some only when the recipe is guarded by a predicate."#,
        isa.name
    ));
    fmtln!(
        fmt,
        "pub static RECIPE_PREDICATES: [RecipePredicate; {}] = [",
        isa.recipes.len()
    );
    fmt.indent(|fmt| {
        for recipe in isa.recipes.values() {
            match (&recipe.isa_predicate, &recipe.inst_predicate) {
                (None, None) => fmt.line("None,"),
                key => fmtln!(fmt, "Some({}),", predicate_names.get(&key).unwrap()),
            }
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();
}

/// Emit private functions for matching instruction predicates as well as a static
/// `INST_PREDICATES` array indexed by predicate number.
fn emit_inst_predicates(isa: &TargetIsa, fmt: &mut Formatter) {
    fmt.comment(format!("{} instruction predicates.", isa.name));
    for (id, instp) in isa.encodings_predicates.iter() {
        fmtln!(fmt, "fn inst_predicate_{}(func: &crate::ir::Function, inst: &crate::ir::InstructionData) -> bool {{", id.index());
        fmt.indent(|fmt| {
            emit_instp(instp, /* has func */ true, fmt);
        });
        fmtln!(fmt, "}");
    }
    fmt.empty_line();

    // Generate the static table.
    fmt.doc_comment(format!(
        r#"{} instruction predicate table.

        One entry per instruction predicate, so the encoding bytecode can embed indexes into this
        table."#,
        isa.name
    ));
    fmtln!(
        fmt,
        "pub static INST_PREDICATES: [InstPredicate; {}] = [",
        isa.encodings_predicates.len()
    );
    fmt.indent(|fmt| {
        for id in isa.encodings_predicates.keys() {
            fmtln!(fmt, "inst_predicate_{},", id.index());
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();
}

/// Emit a table of encoding recipe names keyed by recipe number.
///
/// This is used for pretty-printing encodings.
fn emit_recipe_names(isa: &TargetIsa, fmt: &mut Formatter) {
    fmt.doc_comment(format!(
        r#"{} recipe names, using the same recipe index spaces as the one specified by the
        corresponding binemit file."#,
        isa.name
    ));
    fmtln!(
        fmt,
        "static RECIPE_NAMES: [&str; {}] = [",
        isa.recipes.len()
    );
    fmt.indent(|fmt| {
        for recipe in isa.recipes.values() {
            fmtln!(fmt, r#""{}","#, recipe.name);
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();
}

/// Returns a set of all the registers involved in fixed register constraints.
fn get_fixed_registers(operands_in: &[OperandConstraint]) -> HashSet<Register> {
    HashSet::from_iter(
        operands_in
            .iter()
            .map(|constraint| {
                if let OperandConstraint::FixedReg(reg) = &constraint {
                    Some(*reg)
                } else {
                    None
                }
            })
            .filter(|opt| opt.is_some())
            .map(|opt| opt.unwrap()),
    )
}

/// Emit a struct field initializer for an array of operand constraints.
///
/// Note "fixed_registers" must refer to the other kind of operands (i.e. if we're operating on
/// inputs, fixed_registers must contain the fixed output registers).
fn emit_operand_constraints(
    registers: &IsaRegs,
    recipe: &EncodingRecipe,
    constraints: &[OperandConstraint],
    field_name: &'static str,
    tied_operands: &HashMap<usize, usize>,
    fixed_registers: &HashSet<Register>,
    fmt: &mut Formatter,
) {
    if constraints.is_empty() {
        fmtln!(fmt, "{}: &[],", field_name);
        return;
    }

    fmtln!(fmt, "{}: &[", field_name);
    fmt.indent(|fmt| {
        for (n, constraint) in constraints.iter().enumerate() {
            fmt.line("OperandConstraint {");
            fmt.indent(|fmt| {
                match constraint {
                    OperandConstraint::RegClass(reg_class) => {
                        if let Some(tied_input) = tied_operands.get(&n) {
                            fmtln!(fmt, "kind: ConstraintKind::Tied({}),", tied_input);
                        } else {
                            fmt.line("kind: ConstraintKind::Reg,");
                        }
                        fmtln!(
                            fmt,
                            "regclass: &{}_DATA,",
                            registers.classes[*reg_class].name
                        );
                    }
                    OperandConstraint::FixedReg(reg) => {
                        assert!(!tied_operands.contains_key(&n), "can't tie fixed registers");
                        let constraint_kind = if fixed_registers.contains(&reg) {
                            "FixedTied"
                        } else {
                            "FixedReg"
                        };
                        fmtln!(
                            fmt,
                            "kind: ConstraintKind::{}({}),",
                            constraint_kind,
                            reg.unit
                        );
                        fmtln!(
                            fmt,
                            "regclass: &{}_DATA,",
                            registers.classes[reg.regclass].name
                        );
                    }
                    OperandConstraint::TiedInput(tied_input) => {
                        // This is a tied output constraint. It should never happen
                        // for input constraints.
                        assert!(
                            tied_input == tied_operands.get(&n).unwrap(),
                            "invalid tied constraint"
                        );
                        fmtln!(fmt, "kind: ConstraintKind::Tied({}),", tied_input);

                        let tied_class = if let OperandConstraint::RegClass(tied_class) =
                            recipe.operands_in[*tied_input]
                        {
                            tied_class
                        } else {
                            panic!("tied constraints relate only to register inputs");
                        };

                        fmtln!(
                            fmt,
                            "regclass: &{}_DATA,",
                            registers.classes[tied_class].name
                        );
                    }
                    OperandConstraint::Stack(stack) => {
                        assert!(!tied_operands.contains_key(&n), "can't tie stack operand");
                        fmt.line("kind: ConstraintKind::Stack,");
                        fmtln!(
                            fmt,
                            "regclass: &{}_DATA,",
                            registers.classes[stack.regclass].name
                        );
                    }
                }
            });
            fmt.line("},");
        }
    });
    fmtln!(fmt, "],");
}

/// Emit a table of encoding recipe operand constraints keyed by recipe number.
///
/// These are used by the register allocator to pick registers that can be properly encoded.
fn emit_recipe_constraints(isa: &TargetIsa, fmt: &mut Formatter) {
    fmt.doc_comment(format!(
        r#"{} recipe constraints list, using the same recipe index spaces as the one
        specified by the corresponding binemit file. These constraints are used by register
        allocation to select the right location to use for input and output values."#,
        isa.name
    ));
    fmtln!(
        fmt,
        "static RECIPE_CONSTRAINTS: [RecipeConstraints; {}] = [",
        isa.recipes.len()
    );
    fmt.indent(|fmt| {
        for recipe in isa.recipes.values() {
            // Compute a mapping of tied operands in both directions (input tied to outputs and
            // conversely).
            let mut tied_in_to_out = HashMap::new();
            let mut tied_out_to_in = HashMap::new();
            for (out_index, constraint) in recipe.operands_out.iter().enumerate() {
                if let OperandConstraint::TiedInput(in_index) = &constraint {
                    tied_in_to_out.insert(*in_index, out_index);
                    tied_out_to_in.insert(out_index, *in_index);
                }
            }

            // Find the sets of registers involved in fixed register constraints.
            let fixed_inputs = get_fixed_registers(&recipe.operands_in);
            let fixed_outputs = get_fixed_registers(&recipe.operands_out);

            fmt.comment(format!("Constraints for recipe {}:", recipe.name));
            fmt.line("RecipeConstraints {");
            fmt.indent(|fmt| {
                emit_operand_constraints(
                    &isa.regs,
                    recipe,
                    &recipe.operands_in,
                    "ins",
                    &tied_in_to_out,
                    &fixed_outputs,
                    fmt,
                );
                emit_operand_constraints(
                    &isa.regs,
                    recipe,
                    &recipe.operands_out,
                    "outs",
                    &tied_out_to_in,
                    &fixed_inputs,
                    fmt,
                );
                fmtln!(
                    fmt,
                    "fixed_ins: {},",
                    if !fixed_inputs.is_empty() {
                        "true"
                    } else {
                        "false"
                    }
                );
                fmtln!(
                    fmt,
                    "fixed_outs: {},",
                    if !fixed_outputs.is_empty() {
                        "true"
                    } else {
                        "false"
                    }
                );
                fmtln!(
                    fmt,
                    "tied_ops: {},",
                    if !tied_in_to_out.is_empty() {
                        "true"
                    } else {
                        "false"
                    }
                );
                fmtln!(
                    fmt,
                    "clobbers_flags: {},",
                    if recipe.clobbers_flags {
                        "true"
                    } else {
                        "false"
                    }
                );
            });
            fmt.line("},");
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();
}

/// Emit a table of encoding recipe code size information.
fn emit_recipe_sizing(isa: &TargetIsa, fmt: &mut Formatter) {
    fmt.doc_comment(format!(
        r#"{} recipe sizing descriptors, using the same recipe index spaces as the one
        specified by the corresponding binemit file. These are used to compute the final size of an
        instruction, as well as to compute the range of branches."#,
        isa.name
    ));
    fmtln!(
        fmt,
        "static RECIPE_SIZING: [RecipeSizing; {}] = [",
        isa.recipes.len()
    );
    fmt.indent(|fmt| {
        for recipe in isa.recipes.values() {
            fmt.comment(format!("Code size information for recipe {}:", recipe.name));
            fmt.line("RecipeSizing {");
            fmt.indent(|fmt| {
                fmtln!(fmt, "base_size: {},", recipe.base_size);
                fmtln!(fmt, "compute_size: {},", recipe.compute_size);
                if let Some(range) = &recipe.branch_range {
                    fmtln!(
                        fmt,
                        "branch_range: Some(BranchRange {{ origin: {}, bits: {} }}),",
                        range.inst_size,
                        range.range
                    );
                } else {
                    fmt.line("branch_range: None,");
                }
            });
            fmt.line("},");
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();
}

/// Level 1 table mapping types to `Level2` objects.
struct Level1Table<'cpu_mode> {
    cpu_mode: &'cpu_mode CpuMode,
    legalize_code: TransformGroupIndex,

    table_map: HashMap<Option<ValueType>, usize>,
    table_vec: Vec<Level2Table>,
}

impl<'cpu_mode> Level1Table<'cpu_mode> {
    fn new(cpu_mode: &'cpu_mode CpuMode) -> Self {
        Self {
            cpu_mode,
            legalize_code: cpu_mode.get_default_legalize_code(),
            table_map: HashMap::new(),
            table_vec: Vec::new(),
        }
    }

    /// Returns the level2 table for the given type; None means monomorphic, in this context.
    fn l2table_for(&mut self, typ: Option<ValueType>) -> &mut Level2Table {
        let cpu_mode = &self.cpu_mode;
        let index = match self.table_map.get(&typ) {
            Some(&index) => index,
            None => {
                let legalize_code = cpu_mode.get_legalize_code_for(&typ);
                let table = Level2Table::new(typ.clone(), legalize_code);
                let index = self.table_vec.len();
                self.table_map.insert(typ, index);
                self.table_vec.push(table);
                index
            }
        };
        self.table_vec.get_mut(index).unwrap()
    }

    fn l2tables(&mut self) -> Vec<&mut Level2Table> {
        self.table_vec
            .iter_mut()
            .filter(|table| !table.is_empty())
            .collect::<Vec<_>>()
    }
}

struct Level2HashTableEntry {
    inst_name: String,
    offset: usize,
}

/// Level 2 table mapping instruction opcodes to `EncList` objects.
///
/// A level 2 table can be completely empty if it only holds a custom legalization action for `ty`.
struct Level2Table {
    typ: Option<ValueType>,
    legalize_code: TransformGroupIndex,
    inst_to_encodings: BTreeMap<String, EncodingList>,
    hash_table_offset: Option<usize>,
    hash_table_len: Option<usize>,
}

impl Level2Table {
    fn new(typ: Option<ValueType>, legalize_code: TransformGroupIndex) -> Self {
        Self {
            typ,
            legalize_code,
            inst_to_encodings: BTreeMap::new(),
            hash_table_offset: None,
            hash_table_len: None,
        }
    }

    fn enclist_for(&mut self, inst: &Instruction) -> &mut EncodingList {
        let copied_typ = self.typ.clone();
        self.inst_to_encodings
            .entry(inst.name.clone())
            .or_insert_with(|| EncodingList::new(inst, copied_typ))
    }

    fn enclists(&mut self) -> btree_map::ValuesMut<'_, String, EncodingList> {
        self.inst_to_encodings.values_mut()
    }

    fn is_empty(&self) -> bool {
        self.inst_to_encodings.is_empty()
    }

    fn layout_hashtable(
        &mut self,
        level2_hashtables: &mut Vec<Option<Level2HashTableEntry>>,
        level2_doc: &mut HashMap<usize, Vec<String>>,
    ) {
        let hash_table = generate_table(
            self.inst_to_encodings.values(),
            self.inst_to_encodings.len(),
            // TODO the Python code wanted opcode numbers to start from 1.
            |enc_list| enc_list.inst.opcode_number.index() + 1,
        );

        let hash_table_offset = level2_hashtables.len();
        let hash_table_len = hash_table.len();

        assert!(self.hash_table_offset.is_none());
        assert!(self.hash_table_len.is_none());
        self.hash_table_offset = Some(hash_table_offset);
        self.hash_table_len = Some(hash_table_len);

        level2_hashtables.extend(hash_table.iter().map(|opt_enc_list| {
            opt_enc_list.map(|enc_list| Level2HashTableEntry {
                inst_name: enc_list.inst.camel_name.clone(),
                offset: enc_list.offset.unwrap(),
            })
        }));

        let typ_comment = match &self.typ {
            Some(ty) => ty.to_string(),
            None => "typeless".into(),
        };

        level2_doc.get_or_default(hash_table_offset).push(format!(
            "{:06x}: {}, {} entries",
            hash_table_offset, typ_comment, hash_table_len
        ));
    }
}

/// The u16 values in an encoding list entry are interpreted as follows:
///
/// NR = len(all_recipes)
///
/// entry < 2*NR
///     Try Encoding(entry/2, next_entry) if the recipe predicate is satisfied.
///     If bit 0 is set, stop with the default legalization code.
///     If bit 0 is clear, keep going down the list.
/// entry < PRED_START
///     Stop with legalization code `entry - 2*NR`.
///
/// Remaining entries are interpreted as (skip, pred) pairs, where:
///
/// skip = (entry - PRED_START) >> PRED_BITS
/// pred = (entry - PRED_START) & PRED_MASK
///
/// If the predicate is satisfied, keep going. Otherwise skip over the next
/// `skip` entries. If skip == 0, stop with the default legalization code.
///
/// The `pred` predicate number is interpreted as an instruction predicate if it
/// is in range, otherwise an ISA predicate.

/// Encoding lists are represented as u16 arrays.
const CODE_BITS: usize = 16;

/// Beginning of the predicate code words.
const PRED_START: u16 = 0x1000;

/// Number of bits used to hold a predicate number (instruction + ISA predicates).
const PRED_BITS: usize = 12;

/// Mask for extracting the predicate number.
const PRED_MASK: usize = (1 << PRED_BITS) - 1;

/// Encoder for the list format above.
struct Encoder {
    num_instruction_predicates: usize,

    /// u16 encoding list words.
    words: Vec<u16>,

    /// Documentation comments: Index into `words` + comment.
    docs: Vec<(usize, String)>,
}

impl Encoder {
    fn new(num_instruction_predicates: usize) -> Self {
        Self {
            num_instruction_predicates,
            words: Vec::new(),
            docs: Vec::new(),
        }
    }

    /// Add a recipe+bits entry to the list.
    fn recipe(&mut self, recipes: &Recipes, enc: &Encoding, is_final: bool) {
        let code = (2 * enc.recipe.index() + if is_final { 1 } else { 0 }) as u16;
        assert!(code < PRED_START);

        let doc = format!(
            "--> {}{}",
            enc.to_rust_comment(recipes),
            if is_final { " and stop" } else { "" }
        );
        self.docs.push((self.words.len(), doc));

        self.words.push(code);
        self.words.push(enc.encbits);
    }

    /// Add a predicate entry.
    fn pred(&mut self, pred_comment: String, skip: usize, n: usize) {
        assert!(n <= PRED_MASK);
        let entry = (PRED_START as usize) + (n | (skip << PRED_BITS));
        assert!(entry < (1 << CODE_BITS));
        let entry = entry as u16;

        let doc = if skip == 0 {
            "stop".to_string()
        } else {
            format!("skip {}", skip)
        };
        let doc = format!("{} unless {}", doc, pred_comment);

        self.docs.push((self.words.len(), doc));
        self.words.push(entry);
    }

    /// Add an instruction predicate entry.
    fn inst_predicate(&mut self, pred: InstructionPredicateNumber, skip: usize) {
        let number = pred.index();
        let pred_comment = format!("inst_predicate_{}", number);
        self.pred(pred_comment, skip, number);
    }

    /// Add an ISA predicate entry.
    fn isa_predicate(&mut self, pred: SettingPredicateNumber, skip: usize) {
        // ISA predicates follow the instruction predicates.
        let n = self.num_instruction_predicates + (pred as usize);
        let pred_comment = format!("PredicateView({})", pred);
        self.pred(pred_comment, skip, n);
    }
}

/// List of instructions for encoding a given type + opcode pair.
///
/// An encoding list contains a sequence of predicates and encoding recipes, all encoded as u16
/// values.
struct EncodingList {
    inst: Instruction,
    typ: Option<ValueType>,
    encodings: Vec<Encoding>,
    offset: Option<usize>,
}

impl EncodingList {
    fn new(inst: &Instruction, typ: Option<ValueType>) -> Self {
        Self {
            inst: inst.clone(),
            typ,
            encodings: Default::default(),
            offset: None,
        }
    }

    /// Encode this list as a sequence of u16 numbers.
    ///
    /// Adds the sequence to `enc_lists` and records the returned offset as
    /// `self.offset`.
    ///
    /// Adds comment lines to `enc_lists_doc` keyed by enc_lists offsets.
    fn encode(
        &mut self,
        isa: &TargetIsa,
        cpu_mode: &CpuMode,
        enc_lists: &mut UniqueSeqTable<u16>,
        enc_lists_doc: &mut HashMap<usize, Vec<String>>,
    ) {
        assert!(!self.encodings.is_empty());

        let mut encoder = Encoder::new(isa.encodings_predicates.len());

        let mut index = 0;
        while index < self.encodings.len() {
            let encoding = &self.encodings[index];

            // Try to see how many encodings are following and have the same ISA predicate and
            // instruction predicate, so as to reduce the number of tests carried out by the
            // encoding list interpreter..
            //
            // Encodings with similar tests are hereby called a group. The group includes the
            // current encoding we're looking at.
            let (isa_predicate, inst_predicate) =
                (&encoding.isa_predicate, &encoding.inst_predicate);

            let group_size = {
                let mut group_size = 1;
                while index + group_size < self.encodings.len() {
                    let next_encoding = &self.encodings[index + group_size];
                    if &next_encoding.inst_predicate != inst_predicate
                        || &next_encoding.isa_predicate != isa_predicate
                    {
                        break;
                    }
                    group_size += 1;
                }
                group_size
            };

            let is_last_group = index + group_size == self.encodings.len();

            // The number of entries to skip when a predicate isn't satisfied is the size of both
            // predicates + the size of the group, minus one (for this predicate). Each recipe
            // entry has a size of two u16 (recipe index + bits).
            let mut skip = if is_last_group {
                0
            } else {
                let isap_size = match isa_predicate {
                    Some(_) => 1,
                    None => 0,
                };
                let instp_size = match inst_predicate {
                    Some(_) => 1,
                    None => 0,
                };
                isap_size + instp_size + group_size * 2 - 1
            };

            if let Some(pred) = isa_predicate {
                encoder.isa_predicate(*pred, skip);
                if !is_last_group {
                    skip -= 1;
                }
            }

            if let Some(pred) = inst_predicate {
                encoder.inst_predicate(*pred, skip);
                // No need to update skip, it's dead after this point.
            }

            for i in 0..group_size {
                let encoding = &self.encodings[index + i];
                let is_last_encoding = index + i == self.encodings.len() - 1;
                encoder.recipe(&isa.recipes, encoding, is_last_encoding);
            }

            index += group_size;
        }

        assert!(self.offset.is_none());
        let offset = enc_lists.add(&encoder.words);
        self.offset = Some(offset);

        // Doc comments.
        let recipe_typ_mode_name = format!(
            "{}{} ({})",
            self.inst.name,
            if let Some(typ) = &self.typ {
                format!(".{}", typ.to_string())
            } else {
                "".into()
            },
            cpu_mode.name
        );

        enc_lists_doc
            .get_or_default(offset)
            .push(format!("{:06x}: {}", offset, recipe_typ_mode_name));
        for (pos, doc) in encoder.docs {
            enc_lists_doc.get_or_default(offset + pos).push(doc);
        }
        enc_lists_doc
            .get_or_default(offset + encoder.words.len())
            .insert(0, format!("end of {}", recipe_typ_mode_name));
    }
}

fn make_tables(cpu_mode: &CpuMode) -> Level1Table {
    let mut table = Level1Table::new(cpu_mode);

    for encoding in &cpu_mode.encodings {
        table
            .l2table_for(encoding.bound_type.clone())
            .enclist_for(encoding.inst())
            .encodings
            .push(encoding.clone());
    }

    // Ensure there are level 1 table entries for all types with a custom legalize action.
    for value_type in cpu_mode.get_legalized_types() {
        table.l2table_for(Some(value_type.clone()));
    }
    // ... and also for monomorphic instructions.
    table.l2table_for(None);

    table
}

/// Compute encodings and doc comments for encoding lists in `level1`.
fn encode_enclists(
    isa: &TargetIsa,
    cpu_mode: &CpuMode,
    level1: &mut Level1Table,
    enc_lists: &mut UniqueSeqTable<u16>,
    enc_lists_doc: &mut HashMap<usize, Vec<String>>,
) {
    for level2 in level1.l2tables() {
        for enclist in level2.enclists() {
            enclist.encode(isa, cpu_mode, enc_lists, enc_lists_doc);
        }
    }
}

fn encode_level2_hashtables<'a>(
    level1: &'a mut Level1Table,
    level2_hashtables: &mut Vec<Option<Level2HashTableEntry>>,
    level2_doc: &mut HashMap<usize, Vec<String>>,
) {
    for level2 in level1.l2tables() {
        level2.layout_hashtable(level2_hashtables, level2_doc);
    }
}

fn emit_encoding_tables(defs: &SharedDefinitions, isa: &TargetIsa, fmt: &mut Formatter) {
    // Level 1 tables, one per CPU mode.
    let mut level1_tables: HashMap<&'static str, Level1Table> = HashMap::new();

    // Single table containing all the level2 hash tables.
    let mut level2_hashtables = Vec::new();
    let mut level2_doc: HashMap<usize, Vec<String>> = HashMap::new();

    // Tables for encoding lists with comments.
    let mut enc_lists = UniqueSeqTable::new();
    let mut enc_lists_doc = HashMap::new();

    for cpu_mode in &isa.cpu_modes {
        level2_doc
            .get_or_default(level2_hashtables.len())
            .push(cpu_mode.name.into());

        let mut level1 = make_tables(cpu_mode);

        encode_enclists(
            isa,
            cpu_mode,
            &mut level1,
            &mut enc_lists,
            &mut enc_lists_doc,
        );
        encode_level2_hashtables(&mut level1, &mut level2_hashtables, &mut level2_doc);

        level1_tables.insert(cpu_mode.name, level1);
    }

    // Compute an appropriate Rust integer type to use for offsets into a table of the given length.
    let offset_type = |length: usize| {
        if length <= 0x10000 {
            "u16"
        } else {
            assert!(u32::try_from(length).is_ok(), "table too big!");
            "u32"
        }
    };

    let level1_offset_type = offset_type(level2_hashtables.len());
    let level2_offset_type = offset_type(enc_lists.len());

    // Emit encoding lists.
    fmt.doc_comment(
        format!(r#"{} encoding lists.

        This contains the entire encodings bytecode for every single instruction; the encodings
        interpreter knows where to start from thanks to the initial lookup in the level 1 and level 2
        table entries below."#, isa.name)
    );
    fmtln!(fmt, "pub static ENCLISTS: [u16; {}] = [", enc_lists.len());
    fmt.indent(|fmt| {
        let mut line = Vec::new();
        for (index, entry) in enc_lists.iter().enumerate() {
            if let Some(comments) = enc_lists_doc.get(&index) {
                if !line.is_empty() {
                    fmtln!(fmt, "{},", line.join(", "));
                    line.clear();
                }
                for comment in comments {
                    fmt.comment(comment);
                }
            }
            line.push(format!("{:#06x}", entry));
        }
        if !line.is_empty() {
            fmtln!(fmt, "{},", line.join(", "));
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();

    // Emit the full concatenation of level 2 hash tables.
    fmt.doc_comment(format!(
        r#"{} level 2 hash tables.

        This hash table, keyed by instruction opcode, contains all the starting offsets for the
        encodings interpreter, for all the CPU modes. It is jumped to after a lookup on the
        instruction's controlling type in the level 1 hash table."#,
        isa.name
    ));
    fmtln!(
        fmt,
        "pub static LEVEL2: [Level2Entry<{}>; {}] = [",
        level2_offset_type,
        level2_hashtables.len()
    );
    fmt.indent(|fmt| {
        for (offset, entry) in level2_hashtables.iter().enumerate() {
            if let Some(comments) = level2_doc.get(&offset) {
                for comment in comments {
                    fmt.comment(comment);
                }
            }
            if let Some(entry) = entry {
                fmtln!(
                    fmt,
                    "Level2Entry {{ opcode: Some(crate::ir::Opcode::{}), offset: {:#08x} }},",
                    entry.inst_name,
                    entry.offset
                );
            } else {
                fmt.line("Level2Entry { opcode: None, offset: 0 },");
            }
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();

    // Emit a level 1 hash table for each CPU mode.
    for cpu_mode in &isa.cpu_modes {
        let level1 = &level1_tables.get(cpu_mode.name).unwrap();
        let hash_table = generate_table(
            level1.table_vec.iter(),
            level1.table_vec.len(),
            |level2_table| {
                if let Some(typ) = &level2_table.typ {
                    typ.number().expect("type without a number") as usize
                } else {
                    0
                }
            },
        );

        fmt.doc_comment(format!(
            r#"{} level 1 hash table for the CPU mode {}.

            This hash table, keyed by instruction controlling type, contains all the level 2
            hash-tables offsets for the given CPU mode, as well as a legalization identifier indicating
            which legalization scheme to apply when the instruction doesn't have any valid encoding for
            this CPU mode.
        "#,
            isa.name, cpu_mode.name
        ));
        fmtln!(
            fmt,
            "pub static LEVEL1_{}: [Level1Entry<{}>; {}] = [",
            cpu_mode.name.to_uppercase(),
            level1_offset_type,
            hash_table.len()
        );
        fmt.indent(|fmt| {
            for opt_level2 in hash_table {
                let level2 = match opt_level2 {
                    None => {
                        // Empty hash table entry. Include the default legalization action.
                        fmtln!(fmt, "Level1Entry {{ ty: ir::types::INVALID, log2len: !0, offset: 0, legalize: {} }},",
                               isa.translate_group_index(level1.legalize_code));
                        continue;
                    }
                    Some(level2) => level2,
                };

                let legalize_comment = defs.transform_groups.get(level2.legalize_code).name;
                let legalize_code = isa.translate_group_index(level2.legalize_code);

                let typ_name = if let Some(typ) = &level2.typ {
                    typ.rust_name()
                } else {
                    "ir::types::INVALID".into()
                };

                if level2.is_empty() {
                    // Empty level 2 table: Only a specialized legalization action, no actual
                    // table.
                    // Set an offset that is out of bounds, but make sure it doesn't overflow its
                    // type when adding `1<<log2len`.
                    fmtln!(fmt, "Level1Entry {{ ty: {}, log2len: 0, offset: !0 - 1, legalize: {} }}, // {}",
                           typ_name, legalize_code, legalize_comment);
                    continue;
                }

                // Proper level 2 hash table.
                let l2l = (level2.hash_table_len.unwrap() as f64).log2() as i32;
                assert!(l2l > 0, "Level2 hash table was too small.");
                fmtln!(fmt, "Level1Entry {{ ty: {}, log2len: {}, offset: {:#08x}, legalize: {} }}, // {}",
                       typ_name, l2l, level2.hash_table_offset.unwrap(), legalize_code, legalize_comment);
            }
        });
        fmtln!(fmt, "];");
        fmt.empty_line();
    }
}

fn gen_isa(defs: &SharedDefinitions, isa: &TargetIsa, fmt: &mut Formatter) {
    // Make the `RECIPE_PREDICATES` table.
    emit_recipe_predicates(isa, fmt);

    // Make the `INST_PREDICATES` table.
    emit_inst_predicates(isa, fmt);

    emit_encoding_tables(defs, isa, fmt);

    emit_recipe_names(isa, fmt);
    emit_recipe_constraints(isa, fmt);
    emit_recipe_sizing(isa, fmt);

    // Finally, tie it all together in an `EncInfo`.
    fmt.line("pub static INFO: isa::EncInfo = isa::EncInfo {");
    fmt.indent(|fmt| {
        fmt.line("constraints: &RECIPE_CONSTRAINTS,");
        fmt.line("sizing: &RECIPE_SIZING,");
        fmt.line("names: &RECIPE_NAMES,");
    });
    fmt.line("};");
}

pub(crate) fn generate(
    defs: &SharedDefinitions,
    isa: &TargetIsa,
    filename: &str,
    out_dir: &str,
) -> Result<(), error::Error> {
    let mut fmt = Formatter::new();
    gen_isa(defs, isa, &mut fmt);
    fmt.update_file(filename, out_dir)?;
    Ok(())
}
