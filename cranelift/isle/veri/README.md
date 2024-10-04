# Crocus: An SMT-based ISLE verification tool

This directory contains Crocus, a tool for verifying instruction lowering and transformation rules written in ISLE. Crocus uses an underlying SMT solver to model values in ISLE rules in as logical bitvectors, searching over all possible inputs to find potential soundness counterexamples. The motivation and context project are described in detail in our ASPLOS 2024 paper: [Lightweight, Modular Verification for WebAssembly-to-Native Instruction Selection](https://dl.acm.org/doi/10.1145/3617232.3624862). 

Currently[^1], Crocus requires every ISLE term uses within a rule to have a user-provided specification, or `spec`, that provides the logical preconditions and effects of the term (`require` and `provide` blocks).
The syntax for these specs is embedded as an optional extension to ISLE itself: specs are written in the ISLE source files. 

[^1]: We have work in progress to lower this annotation burden.

## Running on an individual rule

The easiest way to run Crocus on an individual ISLE rule is to give that rule a name. 

For example, to verify the following `aarch64` rule:

```
(rule -1 (lower (has_type (fits_in_64 ty) (band x y)))
    (alu_rs_imm_logic_commutative (ALUOp.And) ty x y))
```

We can add a name (before the priority):
```
(rule band_fits_in_64 -1 (lower (has_type (fits_in_64 ty) (band x y)))
      (alu_rs_imm_logic_commutative (ALUOp.And) ty x y))
```

We also require that the relevant (outermost) CLIF term on the left hand side has a "type instantiation" to specify the types, e.g. bitwidths, we are interested in verifying. In this case, this is provided with:

```
(form
  bv_binary_8_to_64
  ((args (bv  8) (bv  8)) (ret (bv  8)) (canon (bv  8)))
  ((args (bv 16) (bv 16)) (ret (bv 16)) (canon (bv 16)))
  ((args (bv 32) (bv 32)) (ret (bv 32)) (canon (bv 32)))
  ((args (bv 64) (bv 64)) (ret (bv 64)) (canon (bv 64)))
)

(instantiate band bv_binary_8_to_64)
```


We can then invoke the rule with the following, using `-t` or `--term` to specify the relevant CLIF instruction and `--names` to specify the name of the rule:

```
cargo run --  --codegen ../../../codegen --aarch64 -t band --names band_fits_in_64
```

With the expected output:

```
Writing generated file: /Users/avh/research/wasmtime/cranelift/isle/veri/veri_engine/output/clif_opt.isle
Writing generated file: /Users/avh/research/wasmtime/cranelift/isle/veri/veri_engine/output/clif_lower.isle
Verification succeeded for band_fits_in_64, width 8
Verification succeeded for band_fits_in_64, width 16
Verification succeeded for band_fits_in_64, width 32
Verification succeeded for band_fits_in_64, width 64
```

If the rule was unsound, this will report counterexamples. For instance, if we change the rule to the following:

```
(rule band_fits_in_64 -1 (lower (has_type (fits_in_64 ty) (band x y)))
      (alu_rs_imm_logic_commutative (ALUOp.Or) ty x y))
```

Then the output would include counterexamples, like so:

```
Verification failed for band_fits_in_64, width 8
Counterexample summary
(lower (has_type (fits_in_64 [ty|8]) (band [x|#x01|0b00000001] [y|#x00|0b00000000])))
=>
(output_reg (alu_rs_imm_logic_commutative (ALUOp.Orr) [ty|8] [x|#x01|0b00000001] [y|#x00|0b00000000]))

#x00|0b00000000 =>
#x01|0b00000001

Failed condition:
(= ((_ extract 7 0) lower__13) ((_ extract 7 0) output_reg__16))
```

## The annotation language

The annotation maps closely to [SMT-LIB](https://smt-lib.org) theories of bitvectors and booleans, with a several added conveniences. 

### Top-level constructs

We extend the ISLE parser with the following top-level constructs:

- `model` specifies how an ISLE type maps to an SMT type. For example, the follow ISLE type definitions along with their models specify how booleans and `u8`s are modeled:
```
(model u8 (type (bv 8)))
(type u8 (primitive u8))
(model bool (type Bool))
(type bool (primitive bool))
```

Models can be `Bool`, `Int`, or `(bv)` with or without a specific bitwidth. If the bitwidth is not provided, Crocus type inference will verify the rule with all possible inferred widths 

- As in the example above, `instantiate` and `form` specify what type instantiations should be considered for a verification. 

- `spec` terms provide specifications for ISLE declarations, which can correspond to ISLE instructions, ISA instructions, external constructors/extractors defined in Rust, or transient, ISLE-only terms. Specs take the form `(spec (term arg1 ... argN) (provide p1 ... pM) (require r1 ... rO))`, providing the `term` termname (must be a defined ISLE decl), fresh variables `arg1 ... argN` to refer to the arguments, and zero or more provide and require expressions `p1, ..., pN, r1, ..., RN` that take the form of expressions with operations as described below. `spec` terms use the keyword `result` to constrain the return value of the term. 

### General SMT-LIB operations

The following terms exactly match their general SMT-LIB meaning:

- `=`: equality
- `and`: boolean and
- `or`: boolean or
- `not`: boolean negation
- `=>`: boolean implication

We additionally support variadic uses of the `and` and `or` operations (these desugar to the binary SMT-LIB versions as expected). 

### Integer operations

The following terms exactly match the  [SMT-LIB theories `Int`](https://smt-lib.org/theories-Ints.shtml).

- `<`
- `<=`
- `>`
- `>=`

In specs, integer operations are primarily used for comparing the number of bits in an ISLE type.

### Bitvector operations

The following terms exactly match [SMT-LIB theory `FixedSizeBitVectors`](https://smt-lib.org/theories-FixedSizeBitVectors.shtml). 

There operations are typically used in specs for any operations on ISLE `Value`s.

- `bvnot`
- `bvand`
- `bvor`
- `bvxor`
- `bvneg`
- `bvadd`
- `bvsub`
- `bvmul`
- `bvudiv`
- `bvurem`
- `bvsdiv`
- `bvsrem`
- `bvshl`
- `bvlshr`
- `bvashr`
- `bvsaddo`
- `bvule`
- `bvult`
- `bvugt`
- `bvuge`
- `bvslt`
- `bvsle`
- `bvsgt`
- `bvsge`

### Custom bitvector operations

- `int2bv`: equivalent to SMT-LIB `nat2bv`.
- `bv2int`: equivalent to SMT-LIB `bv2nat`.
- `extract`: `(extract h l e)` where `h` and `l` are integer literals and `e` is a bitvector is equivalent to SMT-LIB `((_ extract h l) e)`.
- `zero_ext`: `(zero_ext w e)` where `w : Int` and `e : (bv N)` is equivalent to SMT-LIB `((_ zero_extend M) e))` where `M = w - N`. 
- `sign_ext`: `(sign_ext w e)` where `w : Int` and `e : (bv N)` is equivalent to SMT-LIB `((_ sign_extend M) e))` where `M = w - N`. 
- `rotr`: `(rotr e1 e2)` where `e1, e2: (bv N)` resolves to `(bvor (bvlshr e1 e3) (bvshl e1 (bvsub (nat2bv N N) e3)))`, where `e3 = (bvurem e2 (nat2bv N N))`. Bitvector rotate right.
- `rotl`: `(rotl e1 e2)` where `e1, e2: (bv N)` resolves to `(bvor (bvshl e1 e3) (bvlshr e1 (bvsub (nat2bv N N) e3)))`, where `e3 = (bvurem e2 (nat2bv N N))`. Bitvector rotate left.
- `concat`: `(concat e_1... e_N)` resolves to `(concat e_1 (concat e_2 (concat ... e_N)))`. That is, this is a variadic version of the SMT-LIB `concat` operation. 
- `widthof`: `(widthof e)` where `e : (bv N)` resolves to `N`. That is, returns the bitwidth of a supplied bitvector as an integer. 
- `subs`: `(subs e1 e2)` returns the results of a subtraction with flags. 
- `popcnt`: `(popcnt e)` where `e : (bv N)` returns the count of non-zero bits in `e`.
- `rev`: `(rev e)` where `e : (bv N)` reverses the order of bits in `e`.
- `cls`: `(cls e)` where `e : (bv N)` returns the count of leading sign bits in `e`. 
- `clz`: `(clz e)` where `e : (bv N)` returns the count of leading zero bits in `e`.
- `convto`: `(convto w e)` where `w : Int` and `e : (bv N)` converts the bitvector `e` to the width `w`, leaving the upper bits unspecified in the case of a extension. That is, there are 3 cases:
    1. `w = N`: resolves to `e`.
    2. `w < N`: resolves to `((_ extract M 0) e)` where `M = N - 1`.
    3. `w > N`: resolves to `(concat e2 e)` where `e2` is a fresh bitvector with `w - N` unspecified bits. 

### Custom memory operations

- `load_effect`: `(load_effect flags size address)` where `flags : (bv 16)`, `size: Int`, and `address : (bv 64)` models a load of `size` bits from address `address` with flags `flags`. Only 1 `load_effect` may be used per left hand and right hand side of a rule. 
- `store_effect`: `(store_effect flags size val address)` where `flags : (bv 16)`, `size: Int`, and `val : (bv size)`, `address : (bv 64)` models a store of `val` (with `size` bits) to address `address` with flags `flags`. Only 1 `store_effect` may be used per left hand and right hand side of a rule. 

### Custom control operation

- `if`: equivalent to SMT-LIB `ite`. 
- `switch`: `(switch c (m1 e1) ... (mN eN))` resolves to a series of nested `ite` expressions, 
`(ite(= c m1) e1 (ite (= c m2) e2 (ite ...eN)))`. It additionally adds a verification condition that some case must match, that is, `(or (= c m1) (or (= c m2)...(= c mN)))`.

## Example

Continuing the `band_fits_in_64` example from before, the full required specifications are places in the relevant ISLE files.

```
(rule band_fits_in_64 -1 (lower (has_type (fits_in_64 ty) (band x y)))
      (alu_rs_imm_logic_commutative (ALUOp.And) ty x y))
```

In `inst_specs.isle`:

```
;; The band spec uses the bitvector `bvand` on its arguments.
(spec (band x y)
    (provide (= result (bvand x y))))
(instantiate band bv_binary_8_to_64)
```

In `prelude_lower.isle`:

```
;; has_type checks that the integer modeling the type in matches the Inst bitwidth.
(spec (has_type ty arg)
      (provide (= result arg))
      (require (= ty (widthof arg))))
(decl has_type (Type Inst) Inst)

;; fits_in_64 checks that the integer modeling the width is less than or equal to 64.
(spec (fits_in_64 arg)
      (provide (= result arg))
      (require (<= arg 64)))
(decl fits_in_64 (Type) Type)
```

In `aarch64/lower.isle`: 

```
;; lower is just modeled as an identity function
(spec (lower arg) (provide (= result arg)))
(decl partial lower (Inst) InstOutput)
```

In `aarch64/inst.isle`:

```
;; Enum models ALUOp as an 8-bit bitvector.
(model ALUOp (enum
      (Add #x00) ;; 0
      (Sub #x01)
      (Orr #x02)
      (OrrNot #x03)
      (And #x04)
      (AndNot #x05)
      (Eor #x06)
      (EorNot #x07)
      (SubS #x08)
      (SDiv #x09)
      (UDiv #x0a)
      (RotR #x0b)
      (Lsr #x0c)
      (Asr #x0d)
      (Lsl #x0e)))

;; alu_rs_imm_logic_commutative uses a conv_to and switch. 
(spec (alu_rs_imm_logic_commutative op t a b)
    (provide
      (= result
         (conv_to 64
           (switch op
             ((ALUOp.Orr) (bvor a b))
             ((ALUOp.And) (bvand a b))
             ((ALUOp.Eor) (bvxor a b)))))))
(decl alu_rs_imm_logic_commutative (ALUOp Type Value Value) Reg)
```

## Testing

To see an all of our current output, run tests without capturing standard out:
```bash
cargo test -- --nocapture
```

To run a specific test, you can provide the test name (most rules are tested in `cranelift/isle/veri/veri_engine/tests/veri.rs`). Set `RUST_LOG=DEBUG` to see more detailed output on test cases that expect success.

```bash
RUST_LOG=DEBUG cargo test test_named_band_fits_in_64 -- --nocapture  
```

To see the x86-64 CVE repro, run:

```bash
RUST_LOG=debug cargo run -- --codegen ../../../codegen --noprelude -t amode_add -i examples/x86/amode_add_uextend_shl.isle
```

To see the x86-64 CVE variant with a 32-bit address, run:
```bash
RUST_LOG=debug cargo run --  --codegen ../../../codegen --noprelude -t amode_add -i examples/x86/amode_add_shl.isle
```