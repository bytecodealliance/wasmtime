# Cranelift ISLE Verification Prototype

This crate is a prototype for verifying Cranelift's ISLE lowering rules using an SMT solver.

Currently, term semantics are specified manually in `src/isle_annotations.rs`. These should be replaces by annotations on ISLE terms that are then parsed to our Verification IR.  

## Running on a file

To run on a `.isle` file, run:

```bash
cargo run -- <path-to-file>
```

Right now, this will check equivalence of all rules that start with `lower` on the left hand side. 
The engine will also include ISLE definitions from these two files:
- `cranelift/codegen/src/prelude.isle`
- `cranelift/codegen/src/prelude_lower.isle`
- `cranelift/codegen/src/clif_lower.isle`


## Testing

To see an examples of our current output, run tests without capturing standard out:
```bash
cargo test -- --nocapture
```