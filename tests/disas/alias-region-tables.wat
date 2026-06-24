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
;; @0043                               v4 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0043                               v5 = load.i64 notrap aligned region4 v4+8
;; @0043                               v10 = load.i64 notrap aligned region3 v4
;; @0043                               v16 = iconst.i64 1
;; @0043                               v17 = bor v3, v16  ; v16 = 1
;; @0043                               v6 = ireduce.i32 v5
;; @0043                               v7 = icmp uge v2, v6
;; @0043                               v14 = iconst.i64 0
;; @0043                               v8 = uextend.i64 v2
;; @0043                               v11 = iconst.i64 3
;; @0043                               v12 = ishl v8, v11  ; v11 = 3
;; @0043                               v13 = iadd v10, v12
;; @0043                               v15 = select_spectre_guard v7, v14, v13  ; v14 = 0
;; @0043                               store user6 aligned region5 v17, v15
;; @0049                               v18 = load.i64 notrap aligned region4 v0+80
;; @0049                               v22 = load.i64 notrap aligned region3 v0+72
;; @0049                               v19 = ireduce.i32 v18
;; @0049                               v20 = icmp uge v2, v19
;; @0049                               v25 = iadd v22, v12
;; @0049                               v27 = select_spectre_guard v20, v14, v25  ; v14 = 0
;; @0049                               store user6 aligned region6 v17, v27
;; @004d                               v43 = iconst.i64 -2
;; @004d                               v44 = band v17, v43  ; v43 = -2
;; @004d                               brif v17, block3(v44), block2
;;
;;                                 block2 cold:
;; @004d                               v46 = iconst.i32 0
;; @004d                               v48 = call fn0(v0, v46, v8)  ; v46 = 0
;; @004d                               jump block3(v48)
;;
;;                                 block3(v45: i64):
;; @004d                               v51 = load.i32 user7 aligned readonly v45+16
;; @004d                               v49 = load.i64 notrap aligned readonly can_move region7 v0+40
;; @004d                               v50 = load.i32 notrap aligned readonly can_move v49
;; @004d                               v52 = icmp eq v51, v50
;; @004d                               trapz v52, user8
;; @004d                               v54 = load.i64 notrap aligned readonly v45+8
;; @004d                               v55 = load.i64 notrap aligned readonly v45+24
;; @004d                               v56 = call_indirect sig0, v54(v55, v0)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v56
;; }
