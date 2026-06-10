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
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @002b                               v11 = iconst.i64 0
;; @002b                               v13 = load.i64 user6 aligned region3 v11  ; v11 = 0
;; @002b                               v14 = iconst.i64 -2
;; @002b                               v15 = band v13, v14  ; v14 = -2
;; @002b                               brif v13, block3(v15), block2
;;
;;                                 block2 cold:
;; @002b                               v4 = iconst.i32 0
;; @002b                               v6 = uextend.i64 v3
;; @002b                               v19 = call fn0(v0, v4, v6)  ; v4 = 0
;; @002b                               jump block3(v19)
;;
;;                                 block3(v16: i64):
;; @002b                               v22 = load.i32 user7 aligned readonly region6 v16+16
;; @002b                               v20 = load.i64 notrap aligned readonly can_move region4 v0+40
;; @002b                               v21 = load.i32 notrap aligned readonly can_move region5 v20
;; @002b                               v23 = icmp eq v22, v21
;; @002b                               trapz v23, user8
;; @002b                               v25 = load.i64 notrap aligned readonly region7 v16+8
;; @002b                               v26 = load.i64 notrap aligned readonly region8 v16+24
;; @002b                               v27 = call_indirect sig0, v25(v26, v0, v2)
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return v27
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) -> i32 tail {
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
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0035                               v11 = iconst.i64 0
;; @0035                               v13 = load.i64 user6 aligned region3 v11  ; v11 = 0
;; @0035                               v14 = iconst.i64 -2
;; @0035                               v15 = band v13, v14  ; v14 = -2
;; @0035                               brif v13, block3(v15), block2
;;
;;                                 block2 cold:
;; @0035                               v4 = iconst.i32 0
;; @0035                               v6 = uextend.i64 v3
;; @0035                               v19 = call fn0(v0, v4, v6)  ; v4 = 0
;; @0035                               jump block3(v19)
;;
;;                                 block3(v16: i64):
;; @0035                               v22 = load.i32 user7 aligned readonly region6 v16+16
;; @0035                               v20 = load.i64 notrap aligned readonly can_move region4 v0+40
;; @0035                               v21 = load.i32 notrap aligned readonly can_move region5 v20
;; @0035                               v23 = icmp eq v22, v21
;; @0035                               trapz v23, user8
;; @0035                               v25 = load.i64 notrap aligned readonly region7 v16+8
;; @0035                               v26 = load.i64 notrap aligned readonly region8 v16+24
;; @0035                               return_call_indirect sig0, v25(v26, v0, v2)
;; }
