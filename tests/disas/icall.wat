;;! target = "x86_64"

(module
  (type $ft (func (param f32) (result i32)))
  (func $foo (export "foo") (param i32 f32) (result i32)
    (call_indirect (type $ft) (local.get 1) (local.get 0))
  )
  (table (;0;) 23 23 funcref)
)

;; function u0:0(i64 vmctx, i64, i32, f32) -> i32 tail {
;;     region0 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     sig0 = (i64 vmctx, i64, f32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: f32):
;; @0033                               v5 = iconst.i32 23
;; @0033                               v6 = icmp uge v2, v5  ; v5 = 23
;; @0033                               v7 = uextend.i64 v2
;; @0033                               v8 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v29 = iconst.i64 3
;; @0033                               v9 = ishl v7, v29  ; v29 = 3
;; @0033                               v10 = iadd v8, v9
;; @0033                               v11 = iconst.i64 0
;; @0033                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0033                               v13 = load.i64 user6 aligned region0 v12
;; @0033                               v14 = iconst.i64 -2
;; @0033                               v15 = band v13, v14  ; v14 = -2
;; @0033                               brif v13, block3(v15), block2
;;
;;                                 block2 cold:
;; @0033                               v17 = iconst.i32 0
;; @0033                               v19 = uextend.i64 v2
;; @0033                               v20 = call fn0(v0, v17, v19)  ; v17 = 0
;; @0033                               jump block3(v20)
;;
;;                                 block3(v16: i64):
;; @0033                               v22 = load.i64 notrap aligned readonly can_move v0+40
;; @0033                               v23 = load.i32 notrap aligned readonly can_move v22
;; @0033                               v24 = load.i32 user7 aligned readonly v16+16
;; @0033                               v25 = icmp eq v24, v23
;; @0033                               trapz v25, user8
;; @0033                               v26 = load.i64 notrap aligned readonly v16+8
;; @0033                               v27 = load.i64 notrap aligned readonly v16+24
;; @0033                               v28 = call_indirect sig0, v26(v27, v0, v3)
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v28
;; }
