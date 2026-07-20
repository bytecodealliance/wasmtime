;;! target = "x86_64"

;; This test checks that we do *not* get the indirect-call caching optimization
;; when it is not enabled, because it is off by default.
;;
;; The key bit in the expectation below is that the call sequence in
;; `u0:3` below goes straight to the bounds-check (v5), lazy-table
;; init (masking of bits with v13), and loading of the funcref fields
;; in block3, with no caching fastpath.

(module
 (table 10 10 funcref)

 (func $f1 (result i32) i32.const 1)
 (func $f2 (result i32) i32.const 2)
 (func $f3 (result i32) i32.const 3)

 (func (export "call_it") (param i32) (result i32)
  local.get 0
  call_indirect (result i32))

 ;; Writing to the table keeps it out of the immutable-table
 ;; signature-check elision, so this test keeps demonstrating the
 ;; full non-cached dispatch sequence it was written for.
 (func (export "mutate") (param i32)
  local.get 0
  ref.func $f1
  table.set 0)

 (elem (i32.const 1) func $f1 $f2 $f3))
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
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
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
;; @005e                               v3 = iconst.i32 10
;; @005e                               v4 = icmp uge v2, v3  ; v3 = 10
;; @005e                               v5 = uextend.i64 v2
;; @005e                               v6 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @005e                               v7 = iconst.i64 3
;; @005e                               v8 = ishl v5, v7  ; v7 = 3
;; @005e                               v9 = iadd v6, v8
;; @005e                               v10 = iconst.i64 0
;; @005e                               v11 = select_spectre_guard v4, v10, v9  ; v10 = 0
;; @005e                               v12 = load.i64 user6 aligned region3 v11
;; @005e                               v13 = iconst.i64 -2
;; @005e                               v14 = band v12, v13  ; v13 = -2
;; @005e                               brif v12, block3(v14), block2
;;
;;                                 block2 cold:
;; @005e                               v16 = iconst.i32 0
;; @005e                               v17 = uextend.i64 v2
;; @005e                               v18 = call fn0(v0, v16, v17)  ; v16 = 0
;; @005e                               jump block3(v18)
;;
;;                                 block3(v15: i64):
;; @005e                               v19 = load.i64 notrap aligned readonly can_move region4 v0+40
;; @005e                               v20 = load.i32 notrap aligned readonly can_move region5 v19
;; @005e                               v21 = load.i32 user7 aligned readonly region6 v15+16
;; @005e                               v22 = icmp eq v21, v20
;; @005e                               v23 = uextend.i32 v22
;; @005e                               trapz v23, user8
;; @005e                               v24 = load.i64 notrap aligned readonly region7 v15+8
;; @005e                               v25 = load.i64 notrap aligned readonly region8 v15+24
;; @005e                               v26 = call_indirect sig0, v24(v25, v0)
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return v26
;; }
;;
;; function u0:4(i64 vmctx, i64, i32) tail {
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
;; @0066                               v3 = iconst.i32 0
;; @0066                               v4 = call fn0(v0, v3)  ; v3 = 0
;; @0068                               v5 = iconst.i32 10
;; @0068                               v6 = icmp uge v2, v5  ; v5 = 10
;; @0068                               v7 = uextend.i64 v2
;; @0068                               v8 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0068                               v9 = iconst.i64 3
;; @0068                               v10 = ishl v7, v9  ; v9 = 3
;; @0068                               v11 = iadd v8, v10
;; @0068                               v12 = iconst.i64 0
;; @0068                               v13 = select_spectre_guard v6, v12, v11  ; v12 = 0
;; @0068                               v14 = iconst.i64 1
;; @0068                               v15 = bor v4, v14  ; v14 = 1
;; @0068                               store user6 aligned region3 v15, v13
;; @006a                               jump block1
;;
;;                                 block1:
;; @006a                               return
;; }
