;;! target = "x86_64"

;; Counterpart to `call-indirect-immutable-elide-null.wat`. The static
;; `elem` segment fully covers the table (`(elem (i32.const 0) func $f1
;; $f2 $f3)`), so a casual look at `precomputed_table_has_no_null_slots`
;; says "no nulls". But a second segment in `Expressions` form
;; (`funcref (item ref.null func)`) cannot be folded into `precomputed`
;; — `try_func_table_init` only folds `Functions`-form segments — so
;; it stays in `table_initialization.segments` and runs at instantiation
;; time. That segment writes a null funcref into in-bounds slot 0.
;;
;; `analyze_table_mutability` marks tables targeted by leftover segments
;; as mutated, which kicks both
;; `try_elide_sig_check_for_immutable_table` and (transitively) the
;; `may_be_null = false` lowering off this site. The compiled call
;; therefore keeps the runtime sig check + funcref-NULL trap, and the
;; runtime correctly traps on the null funcref written by the leftover
;; segment instead of dereferencing it.
;;
;; Soundness motivation: skipping the null check on a slot a leftover
;; segment can null out is a null-deref bug. See PR #2 review threads
;; `discussion_r3193374159` and `discussion_r3193374164` on
;; rebeckerspecialties/wasmtime.

(module
  (table 3 3 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    call_indirect (result i32))

  ;; Foldable: full coverage with concrete funcrefs. Without the
  ;; second segment below, this would trigger both elisions.
  (elem (i32.const 0) func $f1 $f2 $f3)

  ;; Expressions-form segment containing a null. Not foldable, runs
  ;; at instantiation, can null out an in-bounds slot. Marks the
  ;; table mutated, suppressing the elisions.
  (elem (i32.const 0) funcref (item ref.null func)))
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0047                               v3 = iconst.i32 1
;; @0049                               jump block1
;;
;;                                 block1:
;; @0049                               return v3  ; v3 = 1
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @004c                               v3 = iconst.i32 2
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return v3  ; v3 = 2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v3 = iconst.i32 3
;; @0053                               jump block1
;;
;;                                 block1:
;; @0053                               return v3  ; v3 = 3
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0058                               v4 = iconst.i32 3
;; @0058                               v5 = icmp uge v2, v4  ; v4 = 3
;; @0058                               v6 = uextend.i64 v2
;; @0058                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v28 = iconst.i64 3
;; @0058                               v8 = ishl v6, v28  ; v28 = 3
;; @0058                               v9 = iadd v7, v8
;; @0058                               v10 = iconst.i64 0
;; @0058                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0058                               v12 = load.i64 user6 aligned table v11
;;                                     v27 = iconst.i64 -2
;; @0058                               v13 = band v12, v27  ; v27 = -2
;; @0058                               brif v12, block3(v13), block2
;;
;;                                 block2 cold:
;; @0058                               v15 = iconst.i32 0
;; @0058                               v17 = uextend.i64 v2
;; @0058                               v18 = call fn0(v0, v15, v17)  ; v15 = 0
;; @0058                               jump block3(v18)
;;
;;                                 block3(v14: i64):
;; @0058                               v20 = load.i64 notrap aligned readonly can_move v0+40
;; @0058                               v21 = load.i32 notrap aligned readonly can_move v20
;; @0058                               v22 = load.i32 user7 aligned readonly v14+16
;; @0058                               v23 = icmp eq v22, v21
;; @0058                               trapz v23, user8
;; @0058                               v24 = load.i64 notrap aligned readonly v14+8
;; @0058                               v25 = load.i64 notrap aligned readonly v14+24
;; @0058                               v26 = call_indirect sig0, v24(v25, v0)
;; @005b                               jump block1
;;
;;                                 block1:
;; @005b                               return v26
;; }
