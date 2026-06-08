;;! target = "x86_64"

(module
  ;; These two types should be deduped to the same `ir::Signature` in the
  ;; translated CLIF.
  (type (func (result i32)))
  (type (func (result i32)))

  (import "" "" (table 0 funcref))

  (func (param i32) (result i32 i32)
    local.get 0
    call_indirect (type 0)
    local.get 0
    call_indirect (type 1)
  )
)

;; function u0:0(i64 vmctx, i64, i32) -> i32, i32 tail {
;;     region0 = 1073741824 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned gv4
;;     gv6 = load.i64 notrap aligned gv4+8
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002d                               v62 = load.i64 notrap aligned readonly can_move v0+48
;; @002d                               v5 = load.i64 notrap aligned v62+8
;; @002d                               v6 = ireduce.i32 v5
;; @002d                               v7 = icmp uge v2, v6
;; @002d                               v8 = uextend.i64 v2
;; @002d                               v60 = load.i64 notrap aligned readonly can_move v0+48
;; @002d                               v9 = load.i64 notrap aligned v60
;; @002d                               v10 = iconst.i64 3
;; @002d                               v11 = ishl v8, v10  ; v10 = 3
;; @002d                               v12 = iadd v9, v11
;; @002d                               v13 = iconst.i64 0
;; @002d                               v14 = select_spectre_guard v7, v13, v12  ; v13 = 0
;; @002d                               v15 = load.i64 user6 aligned region0 v14
;; @002d                               v16 = iconst.i64 -2
;; @002d                               v17 = band v15, v16  ; v16 = -2
;; @002d                               brif v15, block3(v17), block2
;;
;;                                 block2 cold:
;; @002d                               v19 = iconst.i32 0
;; @002d                               v20 = uextend.i64 v2
;; @002d                               v21 = call fn0(v0, v19, v20)  ; v19 = 0
;; @002d                               jump block3(v21)
;;
;;                                 block3(v18: i64):
;; @002d                               v22 = load.i64 notrap aligned readonly can_move v0+40
;; @002d                               v23 = load.i32 notrap aligned readonly can_move v22
;; @002d                               v24 = load.i32 user7 aligned readonly v18+16
;; @002d                               v25 = icmp eq v24, v23
;; @002d                               v26 = uextend.i32 v25
;; @002d                               trapz v26, user8
;; @002d                               v27 = load.i64 notrap aligned readonly v18+8
;; @002d                               v28 = load.i64 notrap aligned readonly v18+24
;; @002d                               v29 = call_indirect sig0, v27(v28, v0)
;; @0032                               v58 = load.i64 notrap aligned readonly can_move v0+48
;; @0032                               v31 = load.i64 notrap aligned v58+8
;; @0032                               v32 = ireduce.i32 v31
;; @0032                               v33 = icmp.i32 uge v2, v32
;; @0032                               v34 = uextend.i64 v2
;; @0032                               v56 = load.i64 notrap aligned readonly can_move v0+48
;; @0032                               v35 = load.i64 notrap aligned v56
;; @0032                               v36 = iconst.i64 3
;; @0032                               v37 = ishl v34, v36  ; v36 = 3
;; @0032                               v38 = iadd v35, v37
;; @0032                               v39 = iconst.i64 0
;; @0032                               v40 = select_spectre_guard v33, v39, v38  ; v39 = 0
;; @0032                               v41 = load.i64 user6 aligned region0 v40
;; @0032                               v42 = iconst.i64 -2
;; @0032                               v43 = band v41, v42  ; v42 = -2
;; @0032                               brif v41, block5(v43), block4
;;
;;                                 block4 cold:
;; @0032                               v45 = iconst.i32 0
;; @0032                               v46 = uextend.i64 v2
;; @0032                               v47 = call fn0(v0, v45, v46)  ; v45 = 0
;; @0032                               jump block5(v47)
;;
;;                                 block5(v44: i64):
;; @0032                               v48 = load.i64 notrap aligned readonly can_move v0+40
;; @0032                               v49 = load.i32 notrap aligned readonly can_move v48
;; @0032                               v50 = load.i32 user7 aligned readonly v44+16
;; @0032                               v51 = icmp eq v50, v49
;; @0032                               v52 = uextend.i32 v51
;; @0032                               trapz v52, user8
;; @0032                               v53 = load.i64 notrap aligned readonly v44+8
;; @0032                               v54 = load.i64 notrap aligned readonly v44+24
;; @0032                               v55 = call_indirect sig0, v53(v54, v0)
;; @0035                               jump block1
;;
;;                                 block1:
;; @0035                               return v29, v55
;; }
