#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

macro_rules! declare_id {
    (
        $(#[$attr:meta])*
            $name:ident
    ) => {
        $(#[$attr])*
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub usize);
        impl $name {
            /// Get the index of this id.
            pub fn index(self) -> usize {
                self.0
            }
        }
    };
}

pub mod ast;
pub mod codegen;
pub mod compile;
pub mod error;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod sema;
pub mod trie;
