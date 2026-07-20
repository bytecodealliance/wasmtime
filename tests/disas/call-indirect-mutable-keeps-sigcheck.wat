;;! target = "x86_64"

;; Counterpart to `call-indirect-immutable-elide-sig.wat`. Same module
;; shape — same elem segment, same uniform call-site type — but one
;; function writes to the table via `table.set`. That marks the table
;; as mutated and disables sig-check elision.
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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @004d                               v2 = iconst.i32 1
;; @004f                               jump block1
;;
;;                                 block1:
;; @004f                               return v2  ; v2 = 1
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v2 = iconst.i32 2
;; @0054                               jump block1
;;
;;                                 block1:
;; @0054                               return v2  ; v2 = 2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0057                               v2 = iconst.i32 3
;; @0059                               jump block1
;;
;;                                 block1:
;; @0059                               return v2  ; v2 = 3
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 671088640 "VMTableDefinition+0x0"
;;     region3 = 335544320 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
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
;; @0060                               v8 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0060                               v9 = iconst.i64 3
;; @0060                               v10 = ishl v7, v9  ; v9 = 3
;; @0060                               v11 = iadd v8, v10
;; @0060                               v12 = iconst.i64 0
;; @0060                               v13 = select_spectre_guard v6, v12, v11  ; v12 = 0
;; @0060                               v14 = iconst.i64 1
;; @0060                               v15 = bor v4, v14  ; v14 = 1
;; @0060                               store user6 aligned region3 v15, v13
;; @0062                               jump block1
;;
;;                                 block1:
;; @0062                               return
;; }
;;
;; function u0:4(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 671088640 "VMTableDefinition+0x0"
;;     region3 = 335544320 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     region4 = 40 "VMContext+0x28"
;;     region5 = 1677721600 "TypeIdsArray+0x0"
;;     region6 = 1610612752 "VMFuncRef+0x10"
;;     region7 = 1610612744 "VMFuncRef+0x8"
;;     region8 = 1610612760 "VMFuncRef+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0067                               v3 = iconst.i32 10
;; @0067                               v4 = icmp uge v2, v3  ; v3 = 10
;; @0067                               v5 = uextend.i64 v2
;; @0067                               v6 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0067                               v7 = iconst.i64 3
;; @0067                               v8 = ishl v5, v7  ; v7 = 3
;; @0067                               v9 = iadd v6, v8
;; @0067                               v10 = iconst.i64 0
;; @0067                               v11 = select_spectre_guard v4, v10, v9  ; v10 = 0
;; @0067                               v12 = load.i64 user6 aligned region3 v11
;; @0067                               v13 = iconst.i64 -2
;; @0067                               v14 = band v12, v13  ; v13 = -2
;; @0067                               brif v12, block3(v14), block2
;;
;;                                 block2 cold:
;; @0067                               v16 = iconst.i32 0
;; @0067                               v17 = uextend.i64 v2
;; @0067                               v18 = call fn0(v0, v16, v17)  ; v16 = 0
;; @0067                               jump block3(v18)
;;
;;                                 block3(v15: i64):
;; @0067                               v19 = load.i64 notrap aligned readonly can_move region4 v0+40
;; @0067                               v20 = load.i32 notrap aligned readonly can_move region5 v19
;; @0067                               v21 = load.i32 user7 aligned readonly region6 v15+16
;; @0067                               v22 = icmp eq v21, v20
;; @0067                               v23 = uextend.i32 v22
;; @0067                               trapz v23, user8
;; @0067                               v24 = load.i64 notrap aligned readonly region7 v15+8
;; @0067                               v25 = load.i64 notrap aligned readonly region8 v15+24
;; @0067                               v26 = call_indirect sig0, v24(v25, v0)
;; @006a                               jump block1
;;
;;                                 block1:
;; @006a                               return v26
;; }
