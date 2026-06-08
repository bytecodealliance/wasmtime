;;! target = "x86_64"

(module
  (table (export "t") 0 100 funcref)
  (func (export "f") (param i32 i32) (result i32)
    (call_indirect (param i32) (result i32) (local.get 0) (local.get 1))
  )
)

;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     region0 = 1073741824 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+48
;;     gv5 = load.i64 notrap aligned gv3+56
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0035                               v5 = load.i64 notrap aligned v0+56
;; @0035                               v6 = ireduce.i32 v5
;; @0035                               v7 = icmp uge v3, v6
;; @0035                               v8 = uextend.i64 v3
;; @0035                               v9 = load.i64 notrap aligned v0+48
;; @0035                               v10 = iconst.i64 3
;; @0035                               v11 = ishl v8, v10  ; v10 = 3
;; @0035                               v12 = iadd v9, v11
;; @0035                               v13 = iconst.i64 0
;; @0035                               v14 = select_spectre_guard v7, v13, v12  ; v13 = 0
;; @0035                               v15 = load.i64 user6 aligned region0 v14
;; @0035                               v16 = iconst.i64 -2
;; @0035                               v17 = band v15, v16  ; v16 = -2
;; @0035                               brif v15, block3(v17), block2
;;
;;                                 block2 cold:
;; @0035                               v19 = iconst.i32 0
;; @0035                               v20 = uextend.i64 v3
;; @0035                               v21 = call fn0(v0, v19, v20)  ; v19 = 0
;; @0035                               jump block3(v21)
;;
;;                                 block3(v18: i64):
;; @0035                               v22 = load.i64 notrap aligned readonly can_move v0+40
;; @0035                               v23 = load.i32 notrap aligned readonly can_move v22+4
;; @0035                               v24 = load.i32 user7 aligned readonly v18+16
;; @0035                               v25 = icmp eq v24, v23
;; @0035                               v26 = uextend.i32 v25
;; @0035                               trapz v26, user8
;; @0035                               v27 = load.i64 notrap aligned readonly v18+8
;; @0035                               v28 = load.i64 notrap aligned readonly v18+24
;; @0035                               v29 = call_indirect sig0, v27(v28, v0, v2)
;; @0038                               jump block1
;;
;;                                 block1:
;; @0038                               return v29
;; }
