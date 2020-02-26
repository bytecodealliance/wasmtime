//! Generate human-readable lists of instructions and their encodings.
//!
//! To display the generated contents, one could use
//! `find target -name 'listing-x86*' | xargs cat | less` from the top-level project directory.
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Error, Formatter as DisplayFormatter};

use crate::cdsl::encodings::EncodingContent;
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::recipes::{EncodingRecipe, EncodingRecipeNumber, OperandConstraint};
use crate::cdsl::regs::{IsaRegs, RegClass, RegClassIndex};
use crate::error;
use crate::shared::Definitions as SharedDefinitions;
use crate::srcgen::Formatter;

/// Emit a list of all encodings in a human-readable format.
fn emit_listings(_: &SharedDefinitions, isa: &TargetIsa, fmt: &mut Formatter) {
    let mut recipe_counter = BTreeMap::new();
    for cpu_mode in &isa.cpu_modes {
        fmtln!(fmt, "CPU Mode: {}", cpu_mode.name);
        fmtln!(fmt, "==================");

        // Sort encodings alphabetically.
        let mut sorted_encodings = cpu_mode.encodings.clone();
        sorted_encodings.sort_by(|a, b| a.inst().name.cmp(&b.inst().name));

        // Print each encoding.
        for encoding in &sorted_encodings {
            let printable = DisplayEncoding::new(encoding, isa);
            fmtln!(fmt, "{}", printable);
            let counter = recipe_counter.entry(encoding.recipe).or_insert(0);
            *counter += 1;
        }
        fmtln!(fmt, "");
    }

    // Print usage counts of used recipes.
    fmtln!(fmt, "Used Recipes");
    fmtln!(fmt, "==================");

    // Sort used recipes by most frequently used.
    let mut sorted_recipes: Vec<_> = recipe_counter.iter().collect();
    sorted_recipes.sort_by(|a, b| b.1.cmp(a.1));

    for (recipe, count) in sorted_recipes {
        let recipe = DisplayRecipe::new(*recipe, isa);
        fmtln!(fmt, "{} uses of recipe({})", count, recipe);
    }
    fmtln!(fmt, "");
}

/// Helper structure for pretty-printing encodings.
struct DisplayEncoding<'a> {
    encoding: &'a EncodingContent,
    isa: &'a TargetIsa,
}

impl<'a> DisplayEncoding<'a> {
    fn new(encoding: &'a EncodingContent, isa: &'a TargetIsa) -> Self {
        Self { encoding, isa }
    }

    fn recipe(&self) -> &EncodingRecipe {
        self.isa
            .recipes
            .get(self.encoding.recipe)
            .expect("a recipe to be attached to each encoding")
    }
}

impl Display for DisplayEncoding<'_> {
    fn fmt(&self, f: &mut DisplayFormatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.encoding.inst().name)?;
        match &self.encoding.bound_type {
            None => {}
            Some(ty) => write!(f, ".{}", ty)?,
        }
        write!(f, ", ")?;
        write!(f, "format={}, ", self.recipe().format)?;
        write!(
            f,
            "recipe({}), ",
            DisplayRecipe::new(self.encoding.recipe, self.isa)
        )
    }
}

/// Helper structure for pretty-printing recipes.
struct DisplayRecipe<'a> {
    recipe: &'a EncodingRecipe,
    isa: &'a TargetIsa,
}

impl<'a> DisplayRecipe<'a> {
    fn new(recipe: EncodingRecipeNumber, isa: &'a TargetIsa) -> Self {
        let recipe = isa
            .recipes
            .get(recipe)
            .expect("a recipe to be attached to each encoding");
        Self { recipe, isa }
    }
}

impl Display for DisplayRecipe<'_> {
    fn fmt(&self, f: &mut DisplayFormatter<'_>) -> Result<(), Error> {
        let args_in: Vec<_> = self
            .recipe
            .operands_in
            .iter()
            .map(|c| DisplayOperandConstraint::new(c, self.isa))
            .collect();
        let args_out: Vec<_> = self
            .recipe
            .operands_out
            .iter()
            .map(|c| DisplayOperandConstraint::new(c, self.isa))
            .collect();

        write!(
            f,
            "name: {}, in: {:?}, out: {:?}",
            self.recipe.name, args_in, args_out,
        )
    }
}

/// Helper structure for pretty-printing operand constraints.
struct DisplayOperandConstraint<'a> {
    constraint: &'a OperandConstraint,
    regs: &'a IsaRegs,
}

impl<'a> DisplayOperandConstraint<'a> {
    fn new(constraint: &'a OperandConstraint, isa: &'a TargetIsa) -> Self {
        let regs = &isa.regs;
        Self { constraint, regs }
    }

    fn reg_class(&self, class: RegClassIndex) -> &RegClass {
        self.regs
            .classes
            .get(class)
            .expect("a valid register class")
    }
}

impl Debug for DisplayOperandConstraint<'_> {
    fn fmt(&self, f: &mut DisplayFormatter<'_>) -> Result<(), Error> {
        match self.constraint {
            OperandConstraint::RegClass(class) => write!(f, "{}", self.reg_class(*class).name),
            OperandConstraint::FixedReg(reg) => {
                write!(f, "{}{}", self.reg_class(reg.regclass).name, reg.unit)
            }
            OperandConstraint::TiedInput(tie) => write!(f, "tied({})", tie),
            OperandConstraint::Stack(_) => write!(f, "stack"),
        }
    }
}

/// Generate the listing files.
pub(crate) fn generate(
    defs: &SharedDefinitions,
    isa: &TargetIsa,
    filename: &str,
    out_dir: &str,
) -> Result<(), error::Error> {
    let mut fmt = Formatter::new();
    emit_listings(defs, isa, &mut fmt);
    fmt.update_file(filename, out_dir)?;
    Ok(())
}
