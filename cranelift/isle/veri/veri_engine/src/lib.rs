use easy_smt::SExpr;

pub mod annotations;
pub mod interp;
pub mod solver;
pub mod termname;
pub mod type_inference;
pub mod verify;

pub const REG_WIDTH: usize = 64;

// Use a distinct with as the maximum width any value should have within type inference
pub const MAX_WIDTH: usize = 2 * REG_WIDTH;

pub const FLAGS_WIDTH: usize = 4;

pub const WIDTHS: [usize; 4] = [8, 16, 32, 64];

pub struct Config {
    pub term: String,
    pub names: Option<Vec<String>>,
    pub distinct_check: bool,

    // Closure arguments: SMT context, arguments to the term, lhs, rhs
    pub custom_verification_condition:
        Option<Box<dyn Fn(&easy_smt::Context, Vec<SExpr>, SExpr, SExpr) -> SExpr>>,
    // Closure arguments: SMT context, arguments to the term
    pub custom_assumptions: Option<Box<dyn Fn(&easy_smt::Context, Vec<SExpr>) -> SExpr>>,
}

impl Config {
    pub fn with_term_and_name(term: &str, name: &str) -> Self {
        Config {
            term: term.to_string(),
            distinct_check: true,
            custom_verification_condition: None,
            custom_assumptions: None,
            names: Some(vec![name.to_string()]),
        }
    }
}
