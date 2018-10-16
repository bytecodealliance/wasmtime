//! Cranelift DSL classes.
//!
//! This module defines the classes that are used to define Cranelift
//! instructions and other entities.

pub mod isa;
pub mod regs;
pub mod types;

/// Convert the string `s` to CamelCase.
fn _camel_case(s: &str) -> String {
    let mut output_chars = String::with_capacity(s.len());

    let mut capitalize = true;
    for curr_char in s.chars() {
        if curr_char == '_' {
            capitalize = true;
        } else {
            if capitalize {
                output_chars.extend(curr_char.to_uppercase());
            } else {
                output_chars.push(curr_char);
            }
            capitalize = false;
        }
    }

    output_chars
}

#[cfg(test)]
mod tests {
    use super::_camel_case as camel_case;

    #[test]
    fn camel_case_works() {
        assert_eq!(camel_case("x"), "X");
        assert_eq!(camel_case("camel_case"), "CamelCase");
    }
}
