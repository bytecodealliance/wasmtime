//! Dynamic IR types

use crate::ir::entities::DynamicType;
use crate::ir::GlobalValue;
use crate::ir::PrimaryMap;
use crate::ir::Type;

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

/// A dynamic type object which has a base vector type and a scaling factor.
#[derive(Clone)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct DynamicTypeData {
    /// Base vector type, this is the minimum size of the type.
    pub base_vector_ty: Type,
    /// The dynamic scaling factor of the base vector type.
    pub dynamic_scale: GlobalValue,
}

impl DynamicTypeData {
    /// Create a new dynamic type.
    pub fn new(base_vector_ty: Type, dynamic_scale: GlobalValue) -> Self {
        assert!(base_vector_ty.is_vector());
        Self {
            base_vector_ty,
            dynamic_scale,
        }
    }

    /// Convert 'base_vector_ty' into a concrete dynamic vector type.
    pub fn concrete(&self) -> Option<Type> {
        self.base_vector_ty.vector_to_dynamic()
    }
}

/// All allocated dynamic types.
pub type DynamicTypes = PrimaryMap<DynamicType, DynamicTypeData>;
