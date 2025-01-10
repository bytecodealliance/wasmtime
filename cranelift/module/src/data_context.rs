//! Defines `DataContext`.

use cranelift_codegen::binemit::{Addend, CodeOffset, Reloc};
use cranelift_codegen::entity::PrimaryMap;
use cranelift_codegen::ir;
use std::borrow::ToOwned;
use std::boxed::Box;
use std::string::String;
use std::vec::Vec;

use crate::ModuleRelocTarget;
use crate::module::ModuleReloc;

/// This specifies how data is to be initialized.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub enum Init {
    /// This indicates that no initialization has been specified yet.
    Uninitialized,
    /// Initialize the data with all zeros.
    Zeros {
        /// The size of the data.
        size: usize,
    },
    /// Initialize the data with the specified contents.
    Bytes {
        /// The contents, which also implies the size of the data.
        contents: Box<[u8]>,
    },
}

impl Init {
    /// Return the size of the data to be initialized.
    pub fn size(&self) -> usize {
        match *self {
            Self::Uninitialized => panic!("data size not initialized yet"),
            Self::Zeros { size } => size,
            Self::Bytes { ref contents } => contents.len(),
        }
    }
}

/// A description of a data object.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct DataDescription {
    /// How the data should be initialized.
    pub init: Init,
    /// External function declarations.
    pub function_decls: PrimaryMap<ir::FuncRef, ModuleRelocTarget>,
    /// External data object declarations.
    pub data_decls: PrimaryMap<ir::GlobalValue, ModuleRelocTarget>,
    /// Function addresses to write at specified offsets.
    pub function_relocs: Vec<(CodeOffset, ir::FuncRef)>,
    /// Data addresses to write at specified offsets.
    pub data_relocs: Vec<(CodeOffset, ir::GlobalValue, Addend)>,
    /// Object file section
    pub custom_segment_section: Option<(String, String)>,
    /// Alignment in bytes. `None` means that the default alignment of the respective module should
    /// be used.
    pub align: Option<u64>,
}

impl DataDescription {
    /// Allocate a new `DataDescription`.
    pub fn new() -> Self {
        Self {
            init: Init::Uninitialized,
            function_decls: PrimaryMap::new(),
            data_decls: PrimaryMap::new(),
            function_relocs: vec![],
            data_relocs: vec![],
            custom_segment_section: None,
            align: None,
        }
    }

    /// Clear all data structures in this `DataDescription`.
    pub fn clear(&mut self) {
        self.init = Init::Uninitialized;
        self.function_decls.clear();
        self.data_decls.clear();
        self.function_relocs.clear();
        self.data_relocs.clear();
        self.custom_segment_section = None;
        self.align = None;
    }

    /// Define a zero-initialized object with the given size.
    pub fn define_zeroinit(&mut self, size: usize) {
        debug_assert_eq!(self.init, Init::Uninitialized);
        self.init = Init::Zeros { size };
    }

    /// Define an object initialized with the given contents.
    ///
    /// TODO: Can we avoid a Box here?
    pub fn define(&mut self, contents: Box<[u8]>) {
        debug_assert_eq!(self.init, Init::Uninitialized);
        self.init = Init::Bytes { contents };
    }

    /// Override the segment/section for data, only supported on Object backend
    pub fn set_segment_section(&mut self, seg: &str, sec: &str) {
        self.custom_segment_section = Some((seg.to_owned(), sec.to_owned()))
    }

    /// Set the alignment for data. The alignment must be a power of two.
    pub fn set_align(&mut self, align: u64) {
        assert!(align.is_power_of_two());
        self.align = Some(align);
    }

    /// Declare an external function import.
    ///
    /// Users of the `Module` API generally should call
    /// `Module::declare_func_in_data` instead, as it takes care of generating
    /// the appropriate `ExternalName`.
    pub fn import_function(&mut self, name: ModuleRelocTarget) -> ir::FuncRef {
        self.function_decls.push(name)
    }

    /// Declares a global value import.
    ///
    /// TODO: Rename to import_data?
    ///
    /// Users of the `Module` API generally should call
    /// `Module::declare_data_in_data` instead, as it takes care of generating
    /// the appropriate `ExternalName`.
    pub fn import_global_value(&mut self, name: ModuleRelocTarget) -> ir::GlobalValue {
        self.data_decls.push(name)
    }

    /// Write the address of `func` into the data at offset `offset`.
    pub fn write_function_addr(&mut self, offset: CodeOffset, func: ir::FuncRef) {
        self.function_relocs.push((offset, func))
    }

    /// Write the address of `data` into the data at offset `offset`.
    pub fn write_data_addr(&mut self, offset: CodeOffset, data: ir::GlobalValue, addend: Addend) {
        self.data_relocs.push((offset, data, addend))
    }

    /// An iterator over all relocations of the data object.
    pub fn all_relocs<'a>(
        &'a self,
        pointer_reloc: Reloc,
    ) -> impl Iterator<Item = ModuleReloc> + 'a {
        let func_relocs = self
            .function_relocs
            .iter()
            .map(move |&(offset, id)| ModuleReloc {
                kind: pointer_reloc,
                offset,
                name: self.function_decls[id].clone(),
                addend: 0,
            });
        let data_relocs = self
            .data_relocs
            .iter()
            .map(move |&(offset, id, addend)| ModuleReloc {
                kind: pointer_reloc,
                offset,
                name: self.data_decls[id].clone(),
                addend,
            });
        func_relocs.chain(data_relocs)
    }
}

#[cfg(test)]
mod tests {
    use crate::ModuleRelocTarget;

    use super::{DataDescription, Init};

    #[test]
    fn basic_data_context() {
        let mut data = DataDescription::new();
        assert_eq!(data.init, Init::Uninitialized);
        assert!(data.function_decls.is_empty());
        assert!(data.data_decls.is_empty());
        assert!(data.function_relocs.is_empty());
        assert!(data.data_relocs.is_empty());

        data.define_zeroinit(256);

        let _func_a = data.import_function(ModuleRelocTarget::user(0, 0));
        let func_b = data.import_function(ModuleRelocTarget::user(0, 1));
        let func_c = data.import_function(ModuleRelocTarget::user(0, 2));
        let _data_a = data.import_global_value(ModuleRelocTarget::user(0, 3));
        let data_b = data.import_global_value(ModuleRelocTarget::user(0, 4));

        data.write_function_addr(8, func_b);
        data.write_function_addr(16, func_c);
        data.write_data_addr(32, data_b, 27);

        assert_eq!(data.init, Init::Zeros { size: 256 });
        assert_eq!(data.function_decls.len(), 3);
        assert_eq!(data.data_decls.len(), 2);
        assert_eq!(data.function_relocs.len(), 2);
        assert_eq!(data.data_relocs.len(), 1);

        data.clear();

        assert_eq!(data.init, Init::Uninitialized);
        assert!(data.function_decls.is_empty());
        assert!(data.data_decls.is_empty());
        assert!(data.function_relocs.is_empty());
        assert!(data.data_relocs.is_empty());

        let contents = vec![33, 34, 35, 36];
        let contents_clone = contents.clone();
        data.define(contents.into_boxed_slice());

        assert_eq!(data.init, Init::Bytes {
            contents: contents_clone.into_boxed_slice()
        });
        assert_eq!(data.function_decls.len(), 0);
        assert_eq!(data.data_decls.len(), 0);
        assert_eq!(data.function_relocs.len(), 0);
        assert_eq!(data.data_relocs.len(), 0);
    }
}
