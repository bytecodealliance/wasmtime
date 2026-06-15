;;! target = "x86_64"
;;! test = "optimize"

(module
  (type $ft (func (result i32)))
  (import "env" "table" (table $imported 10 funcref))
  (table $defined 10 funcref)

  (func (export "test") (param i32 funcref) (result i32)
    ;; Set in imported table
    (table.set $imported (local.get 0) (local.get 1))
    ;; Set in defined table
    (table.set $defined (local.get 0) (local.get 1))
    ;; Indirect call through imported table
    (call_indirect $imported (type $ft) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 48 "VMContext+0x30"
;;     region3 = 2684354560 "VMTableDefinition+0x0"
;;     region4 = 2684354568 "VMTableDefinition+0x8"
;;     region5 = 1073741824 "PublicTable"
;;     region6 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     region7 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0043                               v5 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0043                               v6 = load.i64 notrap aligned region4 v5+8
;; @0043                               v11 = load.i64 notrap aligned region3 v5
;; @0043                               v17 = iconst.i64 1
;; @0043                               v18 = bor v3, v17  ; v17 = 1
;; @0043                               v7 = ireduce.i32 v6
;; @0043                               v8 = icmp uge v2, v7
;; @0043                               v15 = iconst.i64 0
;; @0043                               v9 = uextend.i64 v2
;; @0043                               v12 = iconst.i64 3
;; @0043                               v13 = ishl v9, v12  ; v12 = 3
;; @0043                               v14 = iadd v11, v13
;; @0043                               v16 = select_spectre_guard v8, v15, v14  ; v15 = 0
;; @0043                               store user6 aligned region5 v18, v16
;; @0049                               v19 = load.i64 notrap aligned region4 v0+80
;; @0049                               v23 = load.i64 notrap aligned region3 v0+72
;; @0049                               v20 = ireduce.i32 v19
;; @0049                               v21 = icmp uge v2, v20
;; @0049                               v26 = iadd v23, v13
;; @0049                               v28 = select_spectre_guard v21, v15, v26  ; v15 = 0
;; @0049                               store user6 aligned region6 v18, v28
;; @004d                               v44 = iconst.i64 -2
;; @004d                               v45 = band v18, v44  ; v44 = -2
;; @004d                               brif v18, block3(v45), block2
;;
;;                                 block2 cold:
;; @004d                               v47 = iconst.i32 0
;; @004d                               v49 = call fn0(v0, v47, v9)  ; v47 = 0
;; @004d                               jump block3(v49)
;;
;;                                 block3(v46: i64):
;; @004d                               v52 = load.i32 user7 aligned readonly v46+16
;; @004d                               v50 = load.i64 notrap aligned readonly can_move region7 v0+40
;; @004d                               v51 = load.i32 notrap aligned readonly can_move v50
;; @004d                               v53 = icmp eq v52, v51
;; @004d                               trapz v53, user8
;; @004d                               v55 = load.i64 notrap aligned readonly v46+8
;; @004d                               v56 = load.i64 notrap aligned readonly v46+24
;; @004d                               v57 = call_indirect sig0, v55(v56, v0)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v57
;; }
