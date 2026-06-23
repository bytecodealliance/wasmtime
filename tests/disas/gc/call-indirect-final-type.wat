;;! target = "x86_64"
;;! flags = "-W function-references,gc,tail-call"
;;! test = "optimize"

;; `call_indirect` / `return_call_indirect` whose expected type is `final`,
;; which allows us to omit the slow-path for subtype checks.

(module
  (type $f (func (param i32) (result i32)))   ;; final by default
  (table 0 100 funcref)

  (func (param i32 i32) (result i32)
    (call_indirect (type $f) (local.get 0) (local.get 1)))

  (func (param i32 i32) (result i32)
    (return_call_indirect (type $f) (local.get 0) (local.get 1)))
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2684354560 "VMTableDefinition+0x0"
;;     region3 = 2684354568 "VMTableDefinition+0x8"
;;     region4 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     region5 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @002b                               v4 = load.i64 notrap aligned region3 v0+56
;; @002b                               v8 = load.i64 notrap aligned region2 v0+48
;; @002b                               v5 = ireduce.i32 v4
;; @002b                               v6 = icmp uge v3, v5
;; @002b                               v12 = iconst.i64 0
;; @002b                               v7 = uextend.i64 v3
;; @002b                               v9 = iconst.i64 3
;; @002b                               v10 = ishl v7, v9  ; v9 = 3
;; @002b                               v11 = iadd v8, v10
;; @002b                               v13 = select_spectre_guard v6, v12, v11  ; v12 = 0
;; @002b                               v14 = load.i64 user6 aligned region4 v13
;; @002b                               v15 = iconst.i64 -2
;; @002b                               v16 = band v14, v15  ; v15 = -2
;; @002b                               brif v14, block3(v16), block2
;;
;;                                 block2 cold:
;; @002b                               v18 = iconst.i32 0
;; @002b                               v20 = call fn0(v0, v18, v7)  ; v18 = 0
;; @002b                               jump block3(v20)
;;
;;                                 block3(v17: i64):
;; @002b                               v23 = load.i32 user7 aligned readonly v17+16
;; @002b                               v21 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @002b                               v22 = load.i32 notrap aligned readonly can_move v21
;; @002b                               v24 = icmp eq v23, v22
;; @002b                               trapz v24, user8
;; @002b                               v26 = load.i64 notrap aligned readonly v17+8
;; @002b                               v27 = load.i64 notrap aligned readonly v17+24
;; @002b                               v28 = call_indirect sig0, v26(v27, v0, v2)
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return v28
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2684354560 "VMTableDefinition+0x0"
;;     region3 = 2684354568 "VMTableDefinition+0x8"
;;     region4 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     region5 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0035                               v4 = load.i64 notrap aligned region3 v0+56
;; @0035                               v8 = load.i64 notrap aligned region2 v0+48
;; @0035                               v5 = ireduce.i32 v4
;; @0035                               v6 = icmp uge v3, v5
;; @0035                               v12 = iconst.i64 0
;; @0035                               v7 = uextend.i64 v3
;; @0035                               v9 = iconst.i64 3
;; @0035                               v10 = ishl v7, v9  ; v9 = 3
;; @0035                               v11 = iadd v8, v10
;; @0035                               v13 = select_spectre_guard v6, v12, v11  ; v12 = 0
;; @0035                               v14 = load.i64 user6 aligned region4 v13
;; @0035                               v15 = iconst.i64 -2
;; @0035                               v16 = band v14, v15  ; v15 = -2
;; @0035                               brif v14, block3(v16), block2
;;
;;                                 block2 cold:
;; @0035                               v18 = iconst.i32 0
;; @0035                               v20 = call fn0(v0, v18, v7)  ; v18 = 0
;; @0035                               jump block3(v20)
;;
;;                                 block3(v17: i64):
;; @0035                               v23 = load.i32 user7 aligned readonly v17+16
;; @0035                               v21 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0035                               v22 = load.i32 notrap aligned readonly can_move v21
;; @0035                               v24 = icmp eq v23, v22
;; @0035                               trapz v24, user8
;; @0035                               v26 = load.i64 notrap aligned readonly v17+8
;; @0035                               v27 = load.i64 notrap aligned readonly v17+24
;; @0035                               return_call_indirect sig0, v26(v27, v0, v2)
;; }
