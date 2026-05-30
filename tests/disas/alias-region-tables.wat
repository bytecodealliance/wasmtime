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
;;     region0 = 1073741824 "ImportedTable"
;;     region1 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned gv4
;;     gv6 = load.i64 notrap aligned gv4+8
;;     gv7 = load.i64 notrap aligned gv3+72
;;     gv8 = load.i64 notrap aligned gv3+80
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0043                               v63 = load.i64 notrap aligned readonly can_move v0+48
;; @0043                               v5 = load.i64 notrap aligned v63+8
;; @0043                               v9 = load.i64 notrap aligned v63
;;                                     v59 = iconst.i64 1
;; @0043                               v14 = bor v3, v59  ; v59 = 1
;; @0043                               v6 = ireduce.i32 v5
;; @0043                               v7 = icmp uge v2, v6
;; @0043                               v12 = iconst.i64 0
;; @0043                               v8 = uextend.i64 v2
;;                                     v60 = iconst.i64 3
;; @0043                               v10 = ishl v8, v60  ; v60 = 3
;; @0043                               v11 = iadd v9, v10
;; @0043                               v13 = select_spectre_guard v7, v12, v11  ; v12 = 0
;; @0043                               store user6 aligned region0 v14, v13
;; @0049                               v15 = load.i64 notrap aligned v0+80
;; @0049                               v19 = load.i64 notrap aligned v0+72
;; @0049                               v16 = ireduce.i32 v15
;; @0049                               v17 = icmp uge v2, v16
;; @0049                               v21 = iadd v19, v10
;; @0049                               v23 = select_spectre_guard v17, v12, v21  ; v12 = 0
;; @0049                               store user6 aligned region1 v14, v23
;; @004d                               v35 = iconst.i64 -2
;; @004d                               v36 = band v14, v35  ; v35 = -2
;; @004d                               brif v14, block3(v36), block2
;;
;;                                 block2 cold:
;; @004d                               v38 = iconst.i32 0
;; @004d                               v41 = call fn0(v0, v38, v8)  ; v38 = 0
;; @004d                               jump block3(v41)
;;
;;                                 block3(v37: i64):
;; @004d                               v45 = load.i32 user7 aligned readonly v37+16
;; @004d                               v43 = load.i64 notrap aligned readonly can_move v0+40
;; @004d                               v44 = load.i32 notrap aligned readonly can_move v43
;; @004d                               v46 = icmp eq v45, v44
;; @004d                               trapz v46, user8
;; @004d                               v47 = load.i64 notrap aligned readonly v37+8
;; @004d                               v48 = load.i64 notrap aligned readonly v37+24
;; @004d                               v49 = call_indirect sig0, v47(v48, v0)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v49
;; }
