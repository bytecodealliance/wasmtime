//! Defines `Module` and related types.

// TODO: Should `ir::Function` really have a `name`?

// TODO: Factor out `ir::Function`'s `ext_funcs` and `global_values` into a struct
// shared with `DataDescription`?

use super::HashMap;
use crate::data_context::DataDescription;
use core::fmt::Display;
use cranelift_codegen::binemit::{CodeOffset, Reloc};
use cranelift_codegen::entity::{entity_impl, PrimaryMap};
use cranelift_codegen::ir::function::{Function, VersionMarker};
use cranelift_codegen::ir::ExternalName;
use cranelift_codegen::settings::SetError;
use cranelift_codegen::{
    ir, isa, CodegenError, CompileError, Context, FinalizedMachReloc, FinalizedRelocTarget,
};
use cranelift_control::ControlPlane;
use std::borrow::{Cow, ToOwned};
use std::boxed::Box;
use std::string::String;

/// A module relocation.
#[derive(Clone)]
pub struct ModuleReloc {
    /// The offset at which the relocation applies, *relative to the
    /// containing section*.
    pub offset: CodeOffset,
    /// The kind of relocation.
    pub kind: Reloc,
    /// The external symbol / name to which this relocation refers.
    pub name: ModuleRelocTarget,
    /// The addend to add to the symbol value.
    pub addend: i64,
}

impl ModuleReloc {
    /// Converts a `FinalizedMachReloc` produced from a `Function` into a `ModuleReloc`.
    pub fn from_mach_reloc(
        mach_reloc: &FinalizedMachReloc,
        func: &Function,
        func_id: FuncId,
    ) -> Self {
        let name = match mach_reloc.target {
            FinalizedRelocTarget::ExternalName(ExternalName::User(reff)) => {
                let name = &func.params.user_named_funcs()[reff];
                ModuleRelocTarget::user(name.namespace, name.index)
            }
            FinalizedRelocTarget::ExternalName(ExternalName::TestCase(_)) => unimplemented!(),
            FinalizedRelocTarget::ExternalName(ExternalName::LibCall(libcall)) => {
                ModuleRelocTarget::LibCall(libcall)
            }
            FinalizedRelocTarget::ExternalName(ExternalName::KnownSymbol(ks)) => {
                ModuleRelocTarget::KnownSymbol(ks)
            }
            FinalizedRelocTarget::Func(offset) => {
                ModuleRelocTarget::FunctionOffset(func_id, offset)
            }
        };
        Self {
            offset: mach_reloc.offset,
            kind: mach_reloc.kind,
            name,
            addend: mach_reloc.addend,
        }
    }
}

/// A function identifier for use in the `Module` interface.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct FuncId(u32);
entity_impl!(FuncId, "funcid");

/// Function identifiers are namespace 0 in `ir::ExternalName`
impl From<FuncId> for ModuleRelocTarget {
    fn from(id: FuncId) -> Self {
        Self::User {
            namespace: 0,
            index: id.0,
        }
    }
}

impl FuncId {
    /// Get the `FuncId` for the function named by `name`.
    pub fn from_name(name: &ModuleRelocTarget) -> FuncId {
        if let ModuleRelocTarget::User { namespace, index } = name {
            debug_assert_eq!(*namespace, 0);
            FuncId::from_u32(*index)
        } else {
            panic!("unexpected name in DataId::from_name")
        }
    }
}

/// A data object identifier for use in the `Module` interface.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct DataId(u32);
entity_impl!(DataId, "dataid");

/// Data identifiers are namespace 1 in `ir::ExternalName`
impl From<DataId> for ModuleRelocTarget {
    fn from(id: DataId) -> Self {
        Self::User {
            namespace: 1,
            index: id.0,
        }
    }
}

impl DataId {
    /// Get the `DataId` for the data object named by `name`.
    pub fn from_name(name: &ModuleRelocTarget) -> DataId {
        if let ModuleRelocTarget::User { namespace, index } = name {
            debug_assert_eq!(*namespace, 1);
            DataId::from_u32(*index)
        } else {
            panic!("unexpected name in DataId::from_name")
        }
    }
}

/// Linkage refers to where an entity is defined and who can see it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub enum Linkage {
    /// Defined outside of a module.
    Import,
    /// Defined inside the module, but not visible outside it.
    Local,
    /// Defined inside the module, visible outside it, and may be preempted.
    Preemptible,
    /// Defined inside the module, visible inside the current static linkage unit, but not outside.
    ///
    /// A static linkage unit is the combination of all object files passed to a linker to create
    /// an executable or dynamic library.
    Hidden,
    /// Defined inside the module, and visible outside it.
    Export,
}

impl Linkage {
    fn merge(a: Self, b: Self) -> Self {
        match a {
            Self::Export => Self::Export,
            Self::Hidden => match b {
                Self::Export => Self::Export,
                Self::Preemptible => Self::Preemptible,
                _ => Self::Hidden,
            },
            Self::Preemptible => match b {
                Self::Export => Self::Export,
                _ => Self::Preemptible,
            },
            Self::Local => match b {
                Self::Export => Self::Export,
                Self::Hidden => Self::Hidden,
                Self::Preemptible => Self::Preemptible,
                Self::Local | Self::Import => Self::Local,
            },
            Self::Import => b,
        }
    }

    /// Test whether this linkage can have a definition.
    pub fn is_definable(self) -> bool {
        match self {
            Self::Import => false,
            Self::Local | Self::Preemptible | Self::Hidden | Self::Export => true,
        }
    }

    /// Test whether this linkage will have a definition that cannot be preempted.
    pub fn is_final(self) -> bool {
        match self {
            Self::Import | Self::Preemptible => false,
            Self::Local | Self::Hidden | Self::Export => true,
        }
    }
}

/// A declared name may refer to either a function or data declaration
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub enum FuncOrDataId {
    /// When it's a FuncId
    Func(FuncId),
    /// When it's a DataId
    Data(DataId),
}

/// Mapping to `ModuleExtName` is trivial based on the `FuncId` and `DataId` mapping.
impl From<FuncOrDataId> for ModuleRelocTarget {
    fn from(id: FuncOrDataId) -> Self {
        match id {
            FuncOrDataId::Func(funcid) => Self::from(funcid),
            FuncOrDataId::Data(dataid) => Self::from(dataid),
        }
    }
}

/// Information about a function which can be called.
#[derive(Debug)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[allow(missing_docs, reason = "self-describing fields")]
pub struct FunctionDeclaration {
    pub name: Option<String>,
    pub linkage: Linkage,
    pub signature: ir::Signature,
}

impl FunctionDeclaration {
    /// The linkage name of the function.
    ///
    /// Synthesized from the given function id if it is an anonymous function.
    pub fn linkage_name(&self, id: FuncId) -> Cow<'_, str> {
        match &self.name {
            Some(name) => Cow::Borrowed(name),
            // Symbols starting with .L are completely omitted from the symbol table after linking.
            // Using hexadecimal instead of decimal for slightly smaller symbol names and often
            // slightly faster linking.
            None => Cow::Owned(format!(".Lfn{:x}", id.as_u32())),
        }
    }

    fn merge(
        &mut self,
        id: FuncId,
        linkage: Linkage,
        sig: &ir::Signature,
    ) -> Result<(), ModuleError> {
        self.linkage = Linkage::merge(self.linkage, linkage);
        if &self.signature != sig {
            return Err(ModuleError::IncompatibleSignature(
                self.linkage_name(id).into_owned(),
                self.signature.clone(),
                sig.clone(),
            ));
        }
        Ok(())
    }
}

/// Error messages for all `Module` methods
#[derive(Debug)]
pub enum ModuleError {
    /// Indicates an identifier was used before it was declared
    Undeclared(String),

    /// Indicates an identifier was used as data/function first, but then used as the other
    IncompatibleDeclaration(String),

    /// Indicates a function identifier was declared with a
    /// different signature than declared previously
    IncompatibleSignature(String, ir::Signature, ir::Signature),

    /// Indicates an identifier was defined more than once
    DuplicateDefinition(String),

    /// Indicates an identifier was defined, but was declared as an import
    InvalidImportDefinition(String),

    /// Wraps a `cranelift-codegen` error
    Compilation(CodegenError),

    /// Memory allocation failure from a backend
    Allocation {
        /// Tell where the allocation came from
        message: &'static str,
        /// Io error the allocation failed with
        err: std::io::Error,
    },

    /// Wraps a generic error from a backend
    Backend(anyhow::Error),

    /// Wraps an error from a flag definition.
    Flag(SetError),
}

impl<'a> From<CompileError<'a>> for ModuleError {
    fn from(err: CompileError<'a>) -> Self {
        Self::Compilation(err.inner)
    }
}

// This is manually implementing Error and Display instead of using thiserror to reduce the amount
// of dependencies used by Cranelift.
impl std::error::Error for ModuleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Undeclared { .. }
            | Self::IncompatibleDeclaration { .. }
            | Self::IncompatibleSignature { .. }
            | Self::DuplicateDefinition { .. }
            | Self::InvalidImportDefinition { .. } => None,
            Self::Compilation(source) => Some(source),
            Self::Allocation { err: source, .. } => Some(source),
            Self::Backend(source) => Some(&**source),
            Self::Flag(source) => Some(source),
        }
    }
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Undeclared(name) => {
                write!(f, "Undeclared identifier: {name}")
            }
            Self::IncompatibleDeclaration(name) => {
                write!(f, "Incompatible declaration of identifier: {name}",)
            }
            Self::IncompatibleSignature(name, prev_sig, new_sig) => {
                write!(
                    f,
                    "Function {name} signature {new_sig:?} is incompatible with previous declaration {prev_sig:?}",
                )
            }
            Self::DuplicateDefinition(name) => {
                write!(f, "Duplicate definition of identifier: {name}")
            }
            Self::InvalidImportDefinition(name) => {
                write!(
                    f,
                    "Invalid to define identifier declared as an import: {name}",
                )
            }
            Self::Compilation(err) => {
                write!(f, "Compilation error: {err}")
            }
            Self::Allocation { message, err } => {
                write!(f, "Allocation error: {message}: {err}")
            }
            Self::Backend(err) => write!(f, "Backend error: {err}"),
            Self::Flag(err) => write!(f, "Flag error: {err}"),
        }
    }
}

impl std::convert::From<CodegenError> for ModuleError {
    fn from(source: CodegenError) -> Self {
        Self::Compilation { 0: source }
    }
}

impl std::convert::From<SetError> for ModuleError {
    fn from(source: SetError) -> Self {
        Self::Flag { 0: source }
    }
}

/// A convenient alias for a `Result` that uses `ModuleError` as the error type.
pub type ModuleResult<T> = Result<T, ModuleError>;

/// Information about a data object which can be accessed.
#[derive(Debug)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[allow(missing_docs, reason = "self-describing fields")]
pub struct DataDeclaration {
    pub name: Option<String>,
    pub linkage: Linkage,
    pub writable: bool,
    pub tls: bool,
}

impl DataDeclaration {
    /// The linkage name of the data object.
    ///
    /// Synthesized from the given data id if it is an anonymous function.
    pub fn linkage_name(&self, id: DataId) -> Cow<'_, str> {
        match &self.name {
            Some(name) => Cow::Borrowed(name),
            // Symbols starting with .L are completely omitted from the symbol table after linking.
            // Using hexadecimal instead of decimal for slightly smaller symbol names and often
            // slightly faster linking.
            None => Cow::Owned(format!(".Ldata{:x}", id.as_u32())),
        }
    }

    fn merge(&mut self, linkage: Linkage, writable: bool, tls: bool) {
        self.linkage = Linkage::merge(self.linkage, linkage);
        self.writable = self.writable || writable;
        assert_eq!(
            self.tls, tls,
            "Can't change TLS data object to normal or in the opposite way",
        );
    }
}

/// A translated `ExternalName` into something global we can handle.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub enum ModuleRelocTarget {
    /// User defined function, converted from `ExternalName::User`.
    User {
        /// Arbitrary.
        namespace: u32,
        /// Arbitrary.
        index: u32,
    },
    /// Call into a library function.
    LibCall(ir::LibCall),
    /// Symbols known to the linker.
    KnownSymbol(ir::KnownSymbol),
    /// A offset inside a function
    FunctionOffset(FuncId, CodeOffset),
}

impl ModuleRelocTarget {
    /// Creates a user-defined external name.
    pub fn user(namespace: u32, index: u32) -> Self {
        Self::User { namespace, index }
    }
}

impl Display for ModuleRelocTarget {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::User { namespace, index } => write!(f, "u{namespace}:{index}"),
            Self::LibCall(lc) => write!(f, "%{lc}"),
            Self::KnownSymbol(ks) => write!(f, "{ks}"),
            Self::FunctionOffset(fname, offset) => write!(f, "{fname}+{offset}"),
        }
    }
}

/// This provides a view to the state of a module which allows `ir::ExternalName`s to be translated
/// into `FunctionDeclaration`s and `DataDeclaration`s.
#[derive(Debug, Default)]
pub struct ModuleDeclarations {
    /// A version marker used to ensure that serialized clif ir is never deserialized with a
    /// different version of Cranelift.
    // Note: This must be the first field to ensure that Serde will deserialize it before
    // attempting to deserialize other fields that are potentially changed between versions.
    _version_marker: VersionMarker,

    names: HashMap<String, FuncOrDataId>,
    functions: PrimaryMap<FuncId, FunctionDeclaration>,
    data_objects: PrimaryMap<DataId, DataDeclaration>,
}

#[cfg(feature = "enable-serde")]
mod serialize {
    // This is manually implementing Serialize and Deserialize to avoid serializing the names field,
    // which can be entirely reconstructed from the functions and data_objects fields, saving space.

    use super::*;

    use serde::de::{Deserialize, Deserializer, Error, MapAccess, SeqAccess, Unexpected, Visitor};
    use serde::ser::{Serialize, SerializeStruct, Serializer};
    use std::fmt;

    fn get_names<E: Error>(
        functions: &PrimaryMap<FuncId, FunctionDeclaration>,
        data_objects: &PrimaryMap<DataId, DataDeclaration>,
    ) -> Result<HashMap<String, FuncOrDataId>, E> {
        let mut names = HashMap::new();
        for (func_id, decl) in functions.iter() {
            if let Some(name) = &decl.name {
                let old = names.insert(name.clone(), FuncOrDataId::Func(func_id));
                if old.is_some() {
                    return Err(E::invalid_value(
                        Unexpected::Other("duplicate name"),
                        &"FunctionDeclaration's with no duplicate names",
                    ));
                }
            }
        }
        for (data_id, decl) in data_objects.iter() {
            if let Some(name) = &decl.name {
                let old = names.insert(name.clone(), FuncOrDataId::Data(data_id));
                if old.is_some() {
                    return Err(E::invalid_value(
                        Unexpected::Other("duplicate name"),
                        &"DataDeclaration's with no duplicate names",
                    ));
                }
            }
        }
        Ok(names)
    }

    impl Serialize for ModuleDeclarations {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let ModuleDeclarations {
                _version_marker,
                functions,
                data_objects,
                names: _,
            } = self;

            let mut state = s.serialize_struct("ModuleDeclarations", 4)?;
            state.serialize_field("_version_marker", _version_marker)?;
            state.serialize_field("functions", functions)?;
            state.serialize_field("data_objects", data_objects)?;
            state.end()
        }
    }

    enum ModuleDeclarationsField {
        VersionMarker,
        Functions,
        DataObjects,
        Ignore,
    }

    struct ModuleDeclarationsFieldVisitor;

    impl<'de> serde::de::Visitor<'de> for ModuleDeclarationsFieldVisitor {
        type Value = ModuleDeclarationsField;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("field identifier")
        }

        fn visit_u64<E: Error>(self, val: u64) -> Result<Self::Value, E> {
            match val {
                0u64 => Ok(ModuleDeclarationsField::VersionMarker),
                1u64 => Ok(ModuleDeclarationsField::Functions),
                2u64 => Ok(ModuleDeclarationsField::DataObjects),
                _ => Ok(ModuleDeclarationsField::Ignore),
            }
        }

        fn visit_str<E: Error>(self, val: &str) -> Result<Self::Value, E> {
            match val {
                "_version_marker" => Ok(ModuleDeclarationsField::VersionMarker),
                "functions" => Ok(ModuleDeclarationsField::Functions),
                "data_objects" => Ok(ModuleDeclarationsField::DataObjects),
                _ => Ok(ModuleDeclarationsField::Ignore),
            }
        }

        fn visit_bytes<E: Error>(self, val: &[u8]) -> Result<Self::Value, E> {
            match val {
                b"_version_marker" => Ok(ModuleDeclarationsField::VersionMarker),
                b"functions" => Ok(ModuleDeclarationsField::Functions),
                b"data_objects" => Ok(ModuleDeclarationsField::DataObjects),
                _ => Ok(ModuleDeclarationsField::Ignore),
            }
        }
    }

    impl<'de> Deserialize<'de> for ModuleDeclarationsField {
        #[inline]
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_identifier(ModuleDeclarationsFieldVisitor)
        }
    }

    struct ModuleDeclarationsVisitor;

    impl<'de> Visitor<'de> for ModuleDeclarationsVisitor {
        type Value = ModuleDeclarations;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("struct ModuleDeclarations")
        }

        #[inline]
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let _version_marker = match seq.next_element()? {
                Some(val) => val,
                None => {
                    return Err(Error::invalid_length(
                        0usize,
                        &"struct ModuleDeclarations with 4 elements",
                    ));
                }
            };
            let functions = match seq.next_element()? {
                Some(val) => val,
                None => {
                    return Err(Error::invalid_length(
                        2usize,
                        &"struct ModuleDeclarations with 4 elements",
                    ));
                }
            };
            let data_objects = match seq.next_element()? {
                Some(val) => val,
                None => {
                    return Err(Error::invalid_length(
                        3usize,
                        &"struct ModuleDeclarations with 4 elements",
                    ));
                }
            };
            let names = get_names(&functions, &data_objects)?;
            Ok(ModuleDeclarations {
                _version_marker,
                names,
                functions,
                data_objects,
            })
        }

        #[inline]
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut _version_marker: Option<VersionMarker> = None;
            let mut functions: Option<PrimaryMap<FuncId, FunctionDeclaration>> = None;
            let mut data_objects: Option<PrimaryMap<DataId, DataDeclaration>> = None;
            while let Some(key) = map.next_key::<ModuleDeclarationsField>()? {
                match key {
                    ModuleDeclarationsField::VersionMarker => {
                        if _version_marker.is_some() {
                            return Err(Error::duplicate_field("_version_marker"));
                        }
                        _version_marker = Some(map.next_value()?);
                    }
                    ModuleDeclarationsField::Functions => {
                        if functions.is_some() {
                            return Err(Error::duplicate_field("functions"));
                        }
                        functions = Some(map.next_value()?);
                    }
                    ModuleDeclarationsField::DataObjects => {
                        if data_objects.is_some() {
                            return Err(Error::duplicate_field("data_objects"));
                        }
                        data_objects = Some(map.next_value()?);
                    }
                    _ => {
                        map.next_value::<serde::de::IgnoredAny>()?;
                    }
                }
            }
            let _version_marker = match _version_marker {
                Some(_version_marker) => _version_marker,
                None => return Err(Error::missing_field("_version_marker")),
            };
            let functions = match functions {
                Some(functions) => functions,
                None => return Err(Error::missing_field("functions")),
            };
            let data_objects = match data_objects {
                Some(data_objects) => data_objects,
                None => return Err(Error::missing_field("data_objects")),
            };
            let names = get_names(&functions, &data_objects)?;
            Ok(ModuleDeclarations {
                _version_marker,
                names,
                functions,
                data_objects,
            })
        }
    }

    impl<'de> Deserialize<'de> for ModuleDeclarations {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_struct(
                "ModuleDeclarations",
                &["_version_marker", "functions", "data_objects"],
                ModuleDeclarationsVisitor,
            )
        }
    }
}

impl ModuleDeclarations {
    /// Get the module identifier for a given name, if that name
    /// has been declared.
    pub fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
        self.names.get(name).copied()
    }

    /// Get an iterator of all function declarations
    pub fn get_functions(&self) -> impl Iterator<Item = (FuncId, &FunctionDeclaration)> {
        self.functions.iter()
    }

    /// Return whether `name` names a function, rather than a data object.
    pub fn is_function(name: &ModuleRelocTarget) -> bool {
        match name {
            ModuleRelocTarget::User { namespace, .. } => *namespace == 0,
            ModuleRelocTarget::LibCall(_)
            | ModuleRelocTarget::KnownSymbol(_)
            | ModuleRelocTarget::FunctionOffset(..) => {
                panic!("unexpected module ext name")
            }
        }
    }

    /// Get the `FunctionDeclaration` for the function named by `name`.
    pub fn get_function_decl(&self, func_id: FuncId) -> &FunctionDeclaration {
        &self.functions[func_id]
    }

    /// Get an iterator of all data declarations
    pub fn get_data_objects(&self) -> impl Iterator<Item = (DataId, &DataDeclaration)> {
        self.data_objects.iter()
    }

    /// Get the `DataDeclaration` for the data object named by `name`.
    pub fn get_data_decl(&self, data_id: DataId) -> &DataDeclaration {
        &self.data_objects[data_id]
    }

    /// Declare a function in this module.
    pub fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<(FuncId, Linkage)> {
        // TODO: Can we avoid allocating names so often?
        use super::hash_map::Entry::*;
        match self.names.entry(name.to_owned()) {
            Occupied(entry) => match *entry.get() {
                FuncOrDataId::Func(id) => {
                    let existing = &mut self.functions[id];
                    existing.merge(id, linkage, signature)?;
                    Ok((id, existing.linkage))
                }
                FuncOrDataId::Data(..) => {
                    Err(ModuleError::IncompatibleDeclaration(name.to_owned()))
                }
            },
            Vacant(entry) => {
                let id = self.functions.push(FunctionDeclaration {
                    name: Some(name.to_owned()),
                    linkage,
                    signature: signature.clone(),
                });
                entry.insert(FuncOrDataId::Func(id));
                Ok((id, self.functions[id].linkage))
            }
        }
    }

    /// Declare an anonymous function in this module.
    pub fn declare_anonymous_function(
        &mut self,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        let id = self.functions.push(FunctionDeclaration {
            name: None,
            linkage: Linkage::Local,
            signature: signature.clone(),
        });
        Ok(id)
    }

    /// Declare a data object in this module.
    pub fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<(DataId, Linkage)> {
        // TODO: Can we avoid allocating names so often?
        use super::hash_map::Entry::*;
        match self.names.entry(name.to_owned()) {
            Occupied(entry) => match *entry.get() {
                FuncOrDataId::Data(id) => {
                    let existing = &mut self.data_objects[id];
                    existing.merge(linkage, writable, tls);
                    Ok((id, existing.linkage))
                }

                FuncOrDataId::Func(..) => {
                    Err(ModuleError::IncompatibleDeclaration(name.to_owned()))
                }
            },
            Vacant(entry) => {
                let id = self.data_objects.push(DataDeclaration {
                    name: Some(name.to_owned()),
                    linkage,
                    writable,
                    tls,
                });
                entry.insert(FuncOrDataId::Data(id));
                Ok((id, self.data_objects[id].linkage))
            }
        }
    }

    /// Declare an anonymous data object in this module.
    pub fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        let id = self.data_objects.push(DataDeclaration {
            name: None,
            linkage: Linkage::Local,
            writable,
            tls,
        });
        Ok(id)
    }
}

/// A `Module` is a utility for collecting functions and data objects, and linking them together.
pub trait Module {
    /// Return the `TargetIsa` to compile for.
    fn isa(&self) -> &dyn isa::TargetIsa;

    /// Get all declarations in this module.
    fn declarations(&self) -> &ModuleDeclarations;

    /// Get the module identifier for a given name, if that name
    /// has been declared.
    fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
        self.declarations().get_name(name)
    }

    /// Return the target information needed by frontends to produce Cranelift IR
    /// for the current target.
    fn target_config(&self) -> isa::TargetFrontendConfig {
        self.isa().frontend_config()
    }

    /// Create a new `Context` initialized for use with this `Module`.
    ///
    /// This ensures that the `Context` is initialized with the default calling
    /// convention for the `TargetIsa`.
    fn make_context(&self) -> Context {
        let mut ctx = Context::new();
        ctx.func.signature.call_conv = self.isa().default_call_conv();
        ctx
    }

    /// Clear the given `Context` and reset it for use with a new function.
    ///
    /// This ensures that the `Context` is initialized with the default calling
    /// convention for the `TargetIsa`.
    fn clear_context(&self, ctx: &mut Context) {
        ctx.clear();
        ctx.func.signature.call_conv = self.isa().default_call_conv();
    }

    /// Create a new empty `Signature` with the default calling convention for
    /// the `TargetIsa`, to which parameter and return types can be added for
    /// declaring a function to be called by this `Module`.
    fn make_signature(&self) -> ir::Signature {
        ir::Signature::new(self.isa().default_call_conv())
    }

    /// Clear the given `Signature` and reset for use with a new function.
    ///
    /// This ensures that the `Signature` is initialized with the default
    /// calling convention for the `TargetIsa`.
    fn clear_signature(&self, sig: &mut ir::Signature) {
        sig.clear(self.isa().default_call_conv());
    }

    /// Declare a function in this module.
    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId>;

    /// Declare an anonymous function in this module.
    fn declare_anonymous_function(&mut self, signature: &ir::Signature) -> ModuleResult<FuncId>;

    /// Declare a data object in this module.
    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId>;

    /// Declare an anonymous data object in this module.
    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId>;

    /// Use this when you're building the IR of a function to reference a function.
    ///
    /// TODO: Coalesce redundant decls and signatures.
    /// TODO: Look into ways to reduce the risk of using a FuncRef in the wrong function.
    fn declare_func_in_func(&mut self, func_id: FuncId, func: &mut ir::Function) -> ir::FuncRef {
        let decl = &self.declarations().functions[func_id];
        let signature = func.import_signature(decl.signature.clone());
        let user_name_ref = func.declare_imported_user_function(ir::UserExternalName {
            namespace: 0,
            index: func_id.as_u32(),
        });
        let colocated = decl.linkage.is_final();
        func.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(user_name_ref),
            signature,
            colocated,
        })
    }

    /// Use this when you're building the IR of a function to reference a data object.
    ///
    /// TODO: Same as above.
    fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalValue {
        let decl = &self.declarations().data_objects[data];
        let colocated = decl.linkage.is_final();
        let user_name_ref = func.declare_imported_user_function(ir::UserExternalName {
            namespace: 1,
            index: data.as_u32(),
        });
        func.create_global_value(ir::GlobalValueData::Symbol {
            name: ir::ExternalName::user(user_name_ref),
            offset: ir::immediates::Imm64::new(0),
            colocated,
            tls: decl.tls,
        })
    }

    /// TODO: Same as above.
    fn declare_func_in_data(&self, func_id: FuncId, data: &mut DataDescription) -> ir::FuncRef {
        data.import_function(ModuleRelocTarget::user(0, func_id.as_u32()))
    }

    /// TODO: Same as above.
    fn declare_data_in_data(&self, data_id: DataId, data: &mut DataDescription) -> ir::GlobalValue {
        data.import_global_value(ModuleRelocTarget::user(1, data_id.as_u32()))
    }

    /// Define a function, producing the function body from the given `Context`.
    ///
    /// Returns the size of the function's code and constant data.
    ///
    /// Unlike [`define_function_with_control_plane`] this uses a default [`ControlPlane`] for
    /// convenience.
    ///
    /// Note: After calling this function the given `Context` will contain the compiled function.
    ///
    /// [`define_function_with_control_plane`]: Self::define_function_with_control_plane
    fn define_function(&mut self, func: FuncId, ctx: &mut Context) -> ModuleResult<()> {
        self.define_function_with_control_plane(func, ctx, &mut ControlPlane::default())
    }

    /// Define a function, producing the function body from the given `Context`.
    ///
    /// Returns the size of the function's code and constant data.
    ///
    /// Note: After calling this function the given `Context` will contain the compiled function.
    fn define_function_with_control_plane(
        &mut self,
        func: FuncId,
        ctx: &mut Context,
        ctrl_plane: &mut ControlPlane,
    ) -> ModuleResult<()>;

    /// Define a function, taking the function body from the given `bytes`.
    ///
    /// This function is generally only useful if you need to precisely specify
    /// the emitted instructions for some reason; otherwise, you should use
    /// `define_function`.
    ///
    /// Returns the size of the function's code.
    fn define_function_bytes(
        &mut self,
        func_id: FuncId,
        func: &ir::Function,
        alignment: u64,
        bytes: &[u8],
        relocs: &[FinalizedMachReloc],
    ) -> ModuleResult<()>;

    /// Define a data object, producing the data contents from the given `DataContext`.
    fn define_data(&mut self, data_id: DataId, data: &DataDescription) -> ModuleResult<()>;
}

impl<M: Module + ?Sized> Module for &mut M {
    fn isa(&self) -> &dyn isa::TargetIsa {
        (**self).isa()
    }

    fn declarations(&self) -> &ModuleDeclarations {
        (**self).declarations()
    }

    fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
        (**self).get_name(name)
    }

    fn target_config(&self) -> isa::TargetFrontendConfig {
        (**self).target_config()
    }

    fn make_context(&self) -> Context {
        (**self).make_context()
    }

    fn clear_context(&self, ctx: &mut Context) {
        (**self).clear_context(ctx)
    }

    fn make_signature(&self) -> ir::Signature {
        (**self).make_signature()
    }

    fn clear_signature(&self, sig: &mut ir::Signature) {
        (**self).clear_signature(sig)
    }

    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        (**self).declare_function(name, linkage, signature)
    }

    fn declare_anonymous_function(&mut self, signature: &ir::Signature) -> ModuleResult<FuncId> {
        (**self).declare_anonymous_function(signature)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        (**self).declare_data(name, linkage, writable, tls)
    }

    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        (**self).declare_anonymous_data(writable, tls)
    }

    fn declare_func_in_func(&mut self, func: FuncId, in_func: &mut ir::Function) -> ir::FuncRef {
        (**self).declare_func_in_func(func, in_func)
    }

    fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalValue {
        (**self).declare_data_in_func(data, func)
    }

    fn declare_func_in_data(&self, func_id: FuncId, data: &mut DataDescription) -> ir::FuncRef {
        (**self).declare_func_in_data(func_id, data)
    }

    fn declare_data_in_data(&self, data_id: DataId, data: &mut DataDescription) -> ir::GlobalValue {
        (**self).declare_data_in_data(data_id, data)
    }

    fn define_function(&mut self, func: FuncId, ctx: &mut Context) -> ModuleResult<()> {
        (**self).define_function(func, ctx)
    }

    fn define_function_with_control_plane(
        &mut self,
        func: FuncId,
        ctx: &mut Context,
        ctrl_plane: &mut ControlPlane,
    ) -> ModuleResult<()> {
        (**self).define_function_with_control_plane(func, ctx, ctrl_plane)
    }

    fn define_function_bytes(
        &mut self,
        func_id: FuncId,
        func: &ir::Function,
        alignment: u64,
        bytes: &[u8],
        relocs: &[FinalizedMachReloc],
    ) -> ModuleResult<()> {
        (**self).define_function_bytes(func_id, func, alignment, bytes, relocs)
    }

    fn define_data(&mut self, data_id: DataId, data: &DataDescription) -> ModuleResult<()> {
        (**self).define_data(data_id, data)
    }
}

impl<M: Module + ?Sized> Module for Box<M> {
    fn isa(&self) -> &dyn isa::TargetIsa {
        (**self).isa()
    }

    fn declarations(&self) -> &ModuleDeclarations {
        (**self).declarations()
    }

    fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
        (**self).get_name(name)
    }

    fn target_config(&self) -> isa::TargetFrontendConfig {
        (**self).target_config()
    }

    fn make_context(&self) -> Context {
        (**self).make_context()
    }

    fn clear_context(&self, ctx: &mut Context) {
        (**self).clear_context(ctx)
    }

    fn make_signature(&self) -> ir::Signature {
        (**self).make_signature()
    }

    fn clear_signature(&self, sig: &mut ir::Signature) {
        (**self).clear_signature(sig)
    }

    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        (**self).declare_function(name, linkage, signature)
    }

    fn declare_anonymous_function(&mut self, signature: &ir::Signature) -> ModuleResult<FuncId> {
        (**self).declare_anonymous_function(signature)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        (**self).declare_data(name, linkage, writable, tls)
    }

    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        (**self).declare_anonymous_data(writable, tls)
    }

    fn declare_func_in_func(&mut self, func: FuncId, in_func: &mut ir::Function) -> ir::FuncRef {
        (**self).declare_func_in_func(func, in_func)
    }

    fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalValue {
        (**self).declare_data_in_func(data, func)
    }

    fn declare_func_in_data(&self, func_id: FuncId, data: &mut DataDescription) -> ir::FuncRef {
        (**self).declare_func_in_data(func_id, data)
    }

    fn declare_data_in_data(&self, data_id: DataId, data: &mut DataDescription) -> ir::GlobalValue {
        (**self).declare_data_in_data(data_id, data)
    }

    fn define_function(&mut self, func: FuncId, ctx: &mut Context) -> ModuleResult<()> {
        (**self).define_function(func, ctx)
    }

    fn define_function_with_control_plane(
        &mut self,
        func: FuncId,
        ctx: &mut Context,
        ctrl_plane: &mut ControlPlane,
    ) -> ModuleResult<()> {
        (**self).define_function_with_control_plane(func, ctx, ctrl_plane)
    }

    fn define_function_bytes(
        &mut self,
        func_id: FuncId,
        func: &ir::Function,
        alignment: u64,
        bytes: &[u8],
        relocs: &[FinalizedMachReloc],
    ) -> ModuleResult<()> {
        (**self).define_function_bytes(func_id, func, alignment, bytes, relocs)
    }

    fn define_data(&mut self, data_id: DataId, data: &DataDescription) -> ModuleResult<()> {
        (**self).define_data(data_id, data)
    }
}
