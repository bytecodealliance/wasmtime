;;! target = "x86_64"

;; Counterpart to `call-indirect-immutable-elide-sig.wat`. Same shape —
;; uniform call-site type, no `table.set`/`table.fill`/etc. — but a
;; second elem segment with a dynamic offset (`(global.get $g)` of an
;; imported global) cannot be folded into the precomputed image by
;; `try_func_table_init`. It stays in `table_initialization.segments`
;; and runs at instantiation time, potentially overwriting precomputed
;; slots with a different funcref. `analyze_table_mutability` marks
;; this table as conservatively mutated, which disables sig-check
;; elision in `try_elide_sig_check_for_immutable_table`.
;;
;; Look for the runtime sig load + compare on the call site (the
;; `tables_mutated`-clear elided form is in
;; `call-indirect-immutable-elide-sig.wat`):
;;   load.i32 user6 aligned readonly v_+16
;;   icmp eq
;;   trapz user7
;;
;; Soundness motivation: a leftover segment can introduce a function
;; with a different signature at the same slot, and skipping the sig
;; check would produce type confusion at the call site. See PR #2
;; review threads `discussion_r3193374159` and
;; `discussion_r3193374164` on rebeckerspecialties/wasmtime for the
;; original report.

(module
  (import "" "g" (global $g i32))

  (table 10 10 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    call_indirect (result i32))

  ;; Foldable segment: constant offset, Functions form. This *does*
  ;; populate `precomputed[0..3] = [f1, f2, f3]`.
  (elem (i32.const 0) func $f1 $f2 $f3)

  ;; Leftover segment: dynamic offset → not foldable → stays in
  ;; `table_initialization.segments` → marks the table mutated.
  (elem (offset (global.get $g)) func $f1))
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @004e                               v3 = iconst.i32 1
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v3  ; v3 = 1
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0053                               v3 = iconst.i32 2
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v3  ; v3 = 2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0058                               v3 = iconst.i32 3
;; @005a                               jump block1
;;
;;                                 block1:
;; @005a                               return v3  ; v3 = 3
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+72
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005f                               v4 = iconst.i32 10
;; @005f                               v5 = icmp uge v2, v4  ; v4 = 10
;; @005f                               v6 = uextend.i64 v2
;; @005f                               v7 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v28 = iconst.i64 3
;; @005f                               v8 = ishl v6, v28  ; v28 = 3
;; @005f                               v9 = iadd v7, v8
;; @005f                               v10 = iconst.i64 0
;; @005f                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005f                               v12 = load.i64 user6 aligned table v11
;;                                     v27 = iconst.i64 -2
;; @005f                               v13 = band v12, v27  ; v27 = -2
;; @005f                               brif v12, block3(v13), block2
;;
;;                                 block2 cold:
;; @005f                               v15 = iconst.i32 0
;; @005f                               v17 = uextend.i64 v2
;; @005f                               v18 = call fn0(v0, v15, v17)  ; v15 = 0
;; @005f                               jump block3(v18)
;;
;;                                 block3(v14: i64):
;; @005f                               v20 = load.i64 notrap aligned readonly can_move v0+40
;; @005f                               v21 = load.i32 notrap aligned readonly can_move v20
;; @005f                               v22 = load.i32 user7 aligned readonly v14+16
;; @005f                               v23 = icmp eq v22, v21
;; @005f                               trapz v23, user8
;; @005f                               v24 = load.i64 notrap aligned readonly v14+8
;; @005f                               v25 = load.i64 notrap aligned readonly v14+24
;; @005f                               v26 = call_indirect sig0, v24(v25, v0)
;; @0062                               jump block1
;;
;;                                 block1:
;; @0062                               return v26
;; }
