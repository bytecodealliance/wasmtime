use std::rc::Rc;

use crate::cdsl::instructions::{
    InstSpec, Instruction, InstructionPredicate, InstructionPredicateNode,
    InstructionPredicateNumber, InstructionPredicateRegistry, ValueTypeOrAny,
};
use crate::cdsl::recipes::{EncodingRecipeNumber, Recipes};
use crate::cdsl::settings::SettingPredicateNumber;
use crate::cdsl::types::ValueType;

/// Encoding for a concrete instruction.
///
/// An `Encoding` object ties an instruction opcode with concrete type variables together with an
/// encoding recipe and encoding encbits.
///
/// The concrete instruction can be in three different forms:
///
/// 1. A naked opcode: `trap` for non-polymorphic instructions.
/// 2. With bound type variables: `iadd.i32` for polymorphic instructions.
/// 3. With operands providing constraints: `icmp.i32(intcc.eq, x, y)`.
///
/// If the instruction is polymorphic, all type variables must be provided.
pub struct EncodingContent {
    /// The `Instruction` or `BoundInstruction` being encoded.
    inst: InstSpec,

    /// The `EncodingRecipe` to use.
    pub recipe: EncodingRecipeNumber,

    /// Additional encoding bits to be interpreted by `recipe`.
    pub encbits: u16,

    /// An instruction predicate that must be true to allow selecting this encoding.
    pub inst_predicate: Option<InstructionPredicateNumber>,

    /// An ISA predicate that must be true to allow selecting this encoding.
    pub isa_predicate: Option<SettingPredicateNumber>,

    /// The value type this encoding has been bound to, for encodings of polymorphic instructions.
    pub bound_type: Option<ValueType>,
}

impl EncodingContent {
    pub fn inst(&self) -> &Instruction {
        self.inst.inst()
    }
    pub fn to_rust_comment(&self, recipes: &Recipes) -> String {
        format!("[{}#{:02x}]", recipes[self.recipe].name, self.encbits)
    }
}

pub type Encoding = Rc<EncodingContent>;

pub struct EncodingBuilder {
    inst: InstSpec,
    recipe: EncodingRecipeNumber,
    encbits: u16,
    inst_predicate: Option<InstructionPredicate>,
    isa_predicate: Option<SettingPredicateNumber>,
    bound_type: Option<ValueType>,
}

impl EncodingBuilder {
    pub fn new(inst: InstSpec, recipe: EncodingRecipeNumber, encbits: u16) -> Self {
        let (inst_predicate, bound_type) = match &inst {
            InstSpec::Bound(inst) => {
                let other_typevars = &inst.inst.polymorphic_info.as_ref().unwrap().other_typevars;

                assert!(
                    inst.value_types.len() == other_typevars.len() + 1,
                    "partially bound polymorphic instruction"
                );

                // Add secondary type variables to the instruction predicate.
                let value_types = &inst.value_types;
                let mut inst_predicate = None;
                for (typevar, value_type) in other_typevars.iter().zip(value_types.iter().skip(1)) {
                    let value_type = match value_type {
                        ValueTypeOrAny::Any => continue,
                        ValueTypeOrAny::ValueType(vt) => vt,
                    };
                    let type_predicate =
                        InstructionPredicate::new_typevar_check(&inst.inst, typevar, value_type);
                    inst_predicate = Some(type_predicate.into());
                }

                let ctrl_type = value_types[0]
                    .clone()
                    .expect("Controlling type shouldn't be Any");
                (inst_predicate, Some(ctrl_type))
            }

            InstSpec::Inst(inst) => {
                assert!(
                    inst.polymorphic_info.is_none(),
                    "unbound polymorphic instruction"
                );
                (None, None)
            }
        };

        Self {
            inst,
            recipe,
            encbits,
            inst_predicate,
            isa_predicate: None,
            bound_type,
        }
    }

    pub fn inst_predicate(mut self, inst_predicate: InstructionPredicateNode) -> Self {
        let inst_predicate = Some(match self.inst_predicate {
            Some(node) => node.and(inst_predicate),
            None => inst_predicate.into(),
        });
        self.inst_predicate = inst_predicate;
        self
    }

    pub fn isa_predicate(mut self, isa_predicate: SettingPredicateNumber) -> Self {
        assert!(self.isa_predicate.is_none());
        self.isa_predicate = Some(isa_predicate);
        self
    }

    pub fn build(
        self,
        recipes: &Recipes,
        inst_pred_reg: &mut InstructionPredicateRegistry,
    ) -> Encoding {
        let inst_predicate = self.inst_predicate.map(|pred| inst_pred_reg.insert(pred));

        let inst = self.inst.inst();
        assert!(
            inst.format == recipes[self.recipe].format,
            format!(
                "Inst {} and recipe {} must have the same format!",
                inst.name, recipes[self.recipe].name
            )
        );

        assert_eq!(
            inst.is_branch && !inst.is_indirect_branch,
            recipes[self.recipe].branch_range.is_some(),
            "Inst {}'s is_branch contradicts recipe {} branch_range!",
            inst.name,
            recipes[self.recipe].name
        );

        Rc::new(EncodingContent {
            inst: self.inst,
            recipe: self.recipe,
            encbits: self.encbits,
            inst_predicate,
            isa_predicate: self.isa_predicate,
            bound_type: self.bound_type,
        })
    }
}
