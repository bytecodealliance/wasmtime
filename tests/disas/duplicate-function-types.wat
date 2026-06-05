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
;; @002d                               v66 = load.i64 notrap aligned readonly can_move v0+48
;; @002d                               v5 = load.i64 notrap aligned v66+8
;; @002d                               v6 = ireduce.i32 v5
;; @002d                               v7 = icmp uge v2, v6
;; @002d                               v8 = uextend.i64 v2
;; @002d                               v64 = load.i64 notrap aligned readonly can_move v0+48
;; @002d                               v9 = load.i64 notrap aligned v64
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
;; @002d                               v21 = uextend.i64 v2
;; @002d                               v22 = call fn0(v0, v19, v21)  ; v19 = 0
;; @002d                               jump block3(v22)
;;
;;                                 block3(v18: i64):
;; @002d                               v24 = load.i64 notrap aligned readonly can_move v0+40
;; @002d                               v25 = load.i32 notrap aligned readonly can_move v24
;; @002d                               v26 = load.i32 user7 aligned readonly v18+16
;; @002d                               v27 = icmp eq v26, v25
;; @002d                               v28 = uextend.i32 v27
;; @002d                               trapz v28, user8
;; @002d                               v29 = load.i64 notrap aligned readonly v18+8
;; @002d                               v30 = load.i64 notrap aligned readonly v18+24
;; @002d                               v31 = call_indirect sig0, v29(v30, v0)
;; @0032                               v62 = load.i64 notrap aligned readonly can_move v0+48
;; @0032                               v33 = load.i64 notrap aligned v62+8
;; @0032                               v34 = ireduce.i32 v33
;; @0032                               v35 = icmp.i32 uge v2, v34
;; @0032                               v36 = uextend.i64 v2
;; @0032                               v60 = load.i64 notrap aligned readonly can_move v0+48
;; @0032                               v37 = load.i64 notrap aligned v60
;; @0032                               v38 = iconst.i64 3
;; @0032                               v39 = ishl v36, v38  ; v38 = 3
;; @0032                               v40 = iadd v37, v39
;; @0032                               v41 = iconst.i64 0
;; @0032                               v42 = select_spectre_guard v35, v41, v40  ; v41 = 0
;; @0032                               v43 = load.i64 user6 aligned region0 v42
;; @0032                               v44 = iconst.i64 -2
;; @0032                               v45 = band v43, v44  ; v44 = -2
;; @0032                               brif v43, block5(v45), block4
;;
;;                                 block4 cold:
;; @0032                               v47 = iconst.i32 0
;; @0032                               v49 = uextend.i64 v2
;; @0032                               v50 = call fn0(v0, v47, v49)  ; v47 = 0
;; @0032                               jump block5(v50)
;;
;;                                 block5(v46: i64):
;; @0032                               v52 = load.i64 notrap aligned readonly can_move v0+40
;; @0032                               v53 = load.i32 notrap aligned readonly can_move v52
;; @0032                               v54 = load.i32 user7 aligned readonly v46+16
;; @0032                               v55 = icmp eq v54, v53
;; @0032                               v56 = uextend.i32 v55
;; @0032                               trapz v56, user8
;; @0032                               v57 = load.i64 notrap aligned readonly v46+8
;; @0032                               v58 = load.i64 notrap aligned readonly v46+24
;; @0032                               v59 = call_indirect sig0, v57(v58, v0)
;; @0035                               jump block1
;;
;;                                 block1:
;; @0035                               return v31, v59
;; }
