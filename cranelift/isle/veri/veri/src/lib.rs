// TODO(mbm): declare_id is copied from ISLE crate. move it to a common location?
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

pub mod debug;
pub mod encoded;
pub mod expand;
pub mod explorer;
pub mod program;
pub mod reachability;
pub mod runner;
pub mod solver;
pub mod spec;
pub mod trie;
pub mod type_inference;
pub mod types;
pub mod veri;

#[cfg(test)]
pub mod testing;

include!(concat!(env!("OUT_DIR"), "/meta.rs"));
