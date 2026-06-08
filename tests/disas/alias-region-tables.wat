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
;;     region0 = 1073741824 "PublicTable"
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
;; @0043                               v62 = load.i64 notrap aligned readonly can_move v0+48
;; @0043                               v5 = load.i64 notrap aligned v62+8
;; @0043                               v9 = load.i64 notrap aligned v62
;; @0043                               v15 = iconst.i64 1
;; @0043                               v16 = bor v3, v15  ; v15 = 1
;; @0043                               v6 = ireduce.i32 v5
;; @0043                               v7 = icmp uge v2, v6
;; @0043                               v13 = iconst.i64 0
;; @0043                               v8 = uextend.i64 v2
;; @0043                               v10 = iconst.i64 3
;; @0043                               v11 = ishl v8, v10  ; v10 = 3
;; @0043                               v12 = iadd v9, v11
;; @0043                               v14 = select_spectre_guard v7, v13, v12  ; v13 = 0
;; @0043                               store user6 aligned region0 v16, v14
;; @0049                               v17 = load.i64 notrap aligned v0+80
;; @0049                               v21 = load.i64 notrap aligned v0+72
;; @0049                               v18 = ireduce.i32 v17
;; @0049                               v19 = icmp uge v2, v18
;; @0049                               v24 = iadd v21, v11
;; @0049                               v26 = select_spectre_guard v19, v13, v24  ; v13 = 0
;; @0049                               store user6 aligned region1 v16, v26
;; @004d                               v40 = iconst.i64 -2
;; @004d                               v41 = band v16, v40  ; v40 = -2
;; @004d                               brif v16, block3(v41), block2
;;
;;                                 block2 cold:
;; @004d                               v43 = iconst.i32 0
;; @004d                               v45 = call fn0(v0, v43, v8)  ; v43 = 0
;; @004d                               jump block3(v45)
;;
;;                                 block3(v42: i64):
;; @004d                               v48 = load.i32 user7 aligned readonly v42+16
;; @004d                               v46 = load.i64 notrap aligned readonly can_move v0+40
;; @004d                               v47 = load.i32 notrap aligned readonly can_move v46
;; @004d                               v49 = icmp eq v48, v47
;; @004d                               trapz v49, user8
;; @004d                               v51 = load.i64 notrap aligned readonly v42+8
;; @004d                               v52 = load.i64 notrap aligned readonly v42+24
;; @004d                               v53 = call_indirect sig0, v51(v52, v0)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v53
;; }
