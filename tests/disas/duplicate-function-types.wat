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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned gv4
;;     gv6 = load.i64 notrap aligned gv4+8
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002d                               v64 = load.i64 notrap aligned readonly can_move v0+48
;; @002d                               v5 = load.i64 notrap aligned v64+8
;; @002d                               v6 = ireduce.i32 v5
;; @002d                               v7 = icmp uge v2, v6
;; @002d                               v8 = uextend.i64 v2
;; @002d                               v62 = load.i64 notrap aligned readonly can_move v0+48
;; @002d                               v9 = load.i64 notrap aligned v62
;;                                     v61 = iconst.i64 3
;; @002d                               v10 = ishl v8, v61  ; v61 = 3
;; @002d                               v11 = iadd v9, v10
;; @002d                               v12 = iconst.i64 0
;; @002d                               v13 = select_spectre_guard v7, v12, v11  ; v12 = 0
;; @002d                               v14 = load.i64 user5 aligned table v13
;;                                     v60 = iconst.i64 -2
;; @002d                               v15 = band v14, v60  ; v60 = -2
;; @002d                               brif v14, block3(v15), block2
;;
;;                                 block2 cold:
;; @002d                               v17 = iconst.i32 0
;; @002d                               v19 = uextend.i64 v2
;; @002d                               v20 = call fn0(v0, v17, v19)  ; v17 = 0
;; @002d                               jump block3(v20)
;;
;;                                 block3(v16: i64):
;; @002d                               v22 = load.i64 notrap aligned readonly can_move v0+40
;; @002d                               v23 = load.i32 notrap aligned readonly can_move v22
;; @002d                               v24 = load.i32 user6 aligned readonly v16+16
;; @002d                               v25 = icmp eq v24, v23
;; @002d                               trapz v25, user7
;; @002d                               v26 = load.i64 notrap aligned readonly v16+8
;; @002d                               v27 = load.i64 notrap aligned readonly v16+24
;; @002d                               v28 = call_indirect sig0, v26(v27, v0)
;; @0032                               v58 = load.i64 notrap aligned readonly can_move v0+48
;; @0032                               v30 = load.i64 notrap aligned v58+8
;; @0032                               v31 = ireduce.i32 v30
;; @0032                               v32 = icmp.i32 uge v2, v31
;; @0032                               v33 = uextend.i64 v2
;; @0032                               v56 = load.i64 notrap aligned readonly can_move v0+48
;; @0032                               v34 = load.i64 notrap aligned v56
;;                                     v55 = iconst.i64 3
;; @0032                               v35 = ishl v33, v55  ; v55 = 3
;; @0032                               v36 = iadd v34, v35
;; @0032                               v37 = iconst.i64 0
;; @0032                               v38 = select_spectre_guard v32, v37, v36  ; v37 = 0
;; @0032                               v39 = load.i64 user5 aligned table v38
;;                                     v54 = iconst.i64 -2
;; @0032                               v40 = band v39, v54  ; v54 = -2
;; @0032                               brif v39, block5(v40), block4
;;
;;                                 block4 cold:
;; @0032                               v42 = iconst.i32 0
;; @0032                               v44 = uextend.i64 v2
;; @0032                               v45 = call fn0(v0, v42, v44)  ; v42 = 0
;; @0032                               jump block5(v45)
;;
;;                                 block5(v41: i64):
;; @0032                               v47 = load.i64 notrap aligned readonly can_move v0+40
;; @0032                               v48 = load.i32 notrap aligned readonly can_move v47
;; @0032                               v49 = load.i32 user6 aligned readonly v41+16
;; @0032                               v50 = icmp eq v49, v48
;; @0032                               trapz v50, user7
;; @0032                               v51 = load.i64 notrap aligned readonly v41+8
;; @0032                               v52 = load.i64 notrap aligned readonly v41+24
;; @0032                               v53 = call_indirect sig0, v51(v52, v0)
;; @0035                               jump block1
;;
;;                                 block1:
;; @0035                               return v28, v53
;; }
