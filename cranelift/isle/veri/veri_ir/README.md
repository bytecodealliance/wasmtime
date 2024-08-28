# Verification Intermediate Representation

This crate defines two intermediate representations for verifying ISLE rules.

The core, lower-level Verification IR defined typed expressions for bitvectors, booleans, and integers. 
The higher-level Annotation IR `src/annotation_ir.rs` only requires types on some expressions (currently, constants and function definitions) and has some syntactic sugar for easier bitvector conversions (currently, `VIRExpr::BVConvTo and VIRExpr::BVConvFrom`)