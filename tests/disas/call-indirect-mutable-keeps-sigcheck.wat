;;! target = "x86_64"

;; Counterpart to `call-indirect-immutable-elide-sig.wat`. Same module
;; shape — same elem segment, same uniform call-site type — but one
;; function writes to the table via `table.set`. That sets the
;; `tables_mutated` bit and disables sig-check elision.
;;
;; Look for the runtime sig load + compare on the call site:
;;   load.i32 user6 aligned readonly v_+16
;;   icmp eq
;;   trapz user7
;; (versus the elided form in the immutable test).

(module
  (table 10 10 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  ;; Mutator: this clears the immutability proof for table 0.
  (func (export "mutate") (param i32)
    local.get 0
    ref.func $f1
    table.set 0)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    call_indirect (result i32))

  (elem (i32.const 0) func $f1 $f2 $f3))
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @004d                               v3 = iconst.i32 1
;; @004f                               jump block1
;;
;;                                 block1:
;; @004f                               return v3  ; v3 = 1
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 2
;; @0054                               jump block1
;;
;;                                 block1:
;; @0054                               return v3  ; v3 = 2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0057                               v3 = iconst.i32 3
;; @0059                               jump block1
;;
;;                                 block1:
;; @0059                               return v3  ; v3 = 3
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005e                               v3 = iconst.i32 0
;; @005e                               v5 = call fn0(v0, v3)  ; v3 = 0
;; @0060                               v6 = iconst.i32 10
;; @0060                               v7 = icmp uge v2, v6  ; v6 = 10
;; @0060                               v8 = uextend.i64 v2
;; @0060                               v9 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v16 = iconst.i64 3
;; @0060                               v10 = ishl v8, v16  ; v16 = 3
;; @0060                               v11 = iadd v9, v10
;; @0060                               v12 = iconst.i64 0
;; @0060                               v13 = select_spectre_guard v7, v12, v11  ; v12 = 0
;;                                     v15 = iconst.i64 1
;; @0060                               v14 = bor v5, v15  ; v15 = 1
;; @0060                               store user6 aligned table v14, v13
;; @0062                               jump block1
;;
;;                                 block1:
;; @0062                               return
;; }
;;
;; function u0:4(i64 vmctx, i64, i32) -> i32 tail {
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
;; @0067                               v4 = iconst.i32 10
;; @0067                               v5 = icmp uge v2, v4  ; v4 = 10
;; @0067                               v6 = uextend.i64 v2
;; @0067                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v28 = iconst.i64 3
;; @0067                               v8 = ishl v6, v28  ; v28 = 3
;; @0067                               v9 = iadd v7, v8
;; @0067                               v10 = iconst.i64 0
;; @0067                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0067                               v12 = load.i64 user6 aligned table v11
;;                                     v27 = iconst.i64 -2
;; @0067                               v13 = band v12, v27  ; v27 = -2
;; @0067                               brif v12, block3(v13), block2
;;
;;                                 block2 cold:
;; @0067                               v15 = iconst.i32 0
;; @0067                               v17 = uextend.i64 v2
;; @0067                               v18 = call fn0(v0, v15, v17)  ; v15 = 0
;; @0067                               jump block3(v18)
;;
;;                                 block3(v14: i64):
;; @0067                               v20 = load.i64 notrap aligned readonly can_move v0+40
;; @0067                               v21 = load.i32 notrap aligned readonly can_move v20
;; @0067                               v22 = load.i32 user7 aligned readonly v14+16
;; @0067                               v23 = icmp eq v22, v21
;; @0067                               trapz v23, user8
;; @0067                               v24 = load.i64 notrap aligned readonly v14+8
;; @0067                               v25 = load.i64 notrap aligned readonly v14+24
;; @0067                               v26 = call_indirect sig0, v24(v25, v0)
;; @006a                               jump block1
;;
;;                                 block1:
;; @006a                               return v26
;; }
