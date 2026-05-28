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
;;     region0 = 8 "VMContext+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
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
;;     region0 = 8 "VMContext+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
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
;;     region0 = 8 "VMContext+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005e                               v3 = iconst.i32 0
;; @005e                               v4 = call fn0(v0, v3)  ; v3 = 0
;; @0060                               v5 = iconst.i32 10
;; @0060                               v6 = icmp uge v2, v5  ; v5 = 10
;; @0060                               v7 = uextend.i64 v2
;; @0060                               v8 = load.i64 notrap aligned readonly can_move v0+48
;; @0060                               v9 = iconst.i64 3
;; @0060                               v10 = ishl v7, v9  ; v9 = 3
;; @0060                               v11 = iadd v8, v10
;; @0060                               v12 = iconst.i64 0
;; @0060                               v13 = select_spectre_guard v6, v12, v11  ; v12 = 0
;; @0060                               v14 = iconst.i64 1
;; @0060                               v15 = bor v4, v14  ; v14 = 1
;; @0060                               store user6 aligned region1 v15, v13
;; @0062                               jump block1
;;
;;                                 block1:
;; @0062                               return
;; }
;;
;; function u0:4(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     region2 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
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
;; @0067                               v8 = iconst.i64 3
;; @0067                               v9 = ishl v6, v8  ; v8 = 3
;; @0067                               v10 = iadd v7, v9
;; @0067                               v11 = iconst.i64 0
;; @0067                               v12 = select_spectre_guard v5, v11, v10  ; v11 = 0
;; @0067                               v13 = load.i64 user6 aligned region1 v12
;; @0067                               v14 = iconst.i64 -2
;; @0067                               v15 = band v13, v14  ; v14 = -2
;; @0067                               brif v13, block3(v15), block2
;;
;;                                 block2 cold:
;; @0067                               v17 = iconst.i32 0
;; @0067                               v18 = uextend.i64 v2
;; @0067                               v19 = call fn0(v0, v17, v18)  ; v17 = 0
;; @0067                               jump block3(v19)
;;
;;                                 block3(v16: i64):
;; @0067                               v20 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0067                               v21 = load.i32 notrap aligned readonly can_move v20
;; @0067                               v22 = load.i32 user7 aligned readonly v16+16
;; @0067                               v23 = icmp eq v22, v21
;; @0067                               v24 = uextend.i32 v23
;; @0067                               trapz v24, user8
;; @0067                               v25 = load.i64 notrap aligned readonly v16+8
;; @0067                               v26 = load.i64 notrap aligned readonly v16+24
;; @0067                               v27 = call_indirect sig0, v25(v26, v0)
;; @006a                               jump block1
;;
;;                                 block1:
;; @006a                               return v27
;; }
