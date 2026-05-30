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
;;     region0 = 1073741824 "ImportedTable"
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
;; @002d                               v14 = load.i64 user6 aligned region0 v13
;; @002d                               v15 = iconst.i64 -2
;; @002d                               v16 = band v14, v15  ; v15 = -2
;; @002d                               brif v14, block3(v16), block2
;;
;;                                 block2 cold:
;; @002d                               v18 = iconst.i32 0
;; @002d                               v20 = uextend.i64 v2
;; @002d                               v21 = call fn0(v0, v18, v20)  ; v18 = 0
;; @002d                               jump block3(v21)
;;
;;                                 block3(v17: i64):
;; @002d                               v23 = load.i64 notrap aligned readonly can_move v0+40
;; @002d                               v24 = load.i32 notrap aligned readonly can_move v23
;; @002d                               v25 = load.i32 user7 aligned readonly v17+16
;; @002d                               v26 = icmp eq v25, v24
;; @002d                               trapz v26, user8
;; @002d                               v27 = load.i64 notrap aligned readonly v17+8
;; @002d                               v28 = load.i64 notrap aligned readonly v17+24
;; @002d                               v29 = call_indirect sig0, v27(v28, v0)
;; @0032                               v59 = load.i64 notrap aligned readonly can_move v0+48
;; @0032                               v31 = load.i64 notrap aligned v59+8
;; @0032                               v32 = ireduce.i32 v31
;; @0032                               v33 = icmp.i32 uge v2, v32
;; @0032                               v34 = uextend.i64 v2
;; @0032                               v57 = load.i64 notrap aligned readonly can_move v0+48
;; @0032                               v35 = load.i64 notrap aligned v57
;;                                     v56 = iconst.i64 3
;; @0032                               v36 = ishl v34, v56  ; v56 = 3
;; @0032                               v37 = iadd v35, v36
;; @0032                               v38 = iconst.i64 0
;; @0032                               v39 = select_spectre_guard v33, v38, v37  ; v38 = 0
;; @0032                               v40 = load.i64 user6 aligned region0 v39
;; @0032                               v41 = iconst.i64 -2
;; @0032                               v42 = band v40, v41  ; v41 = -2
;; @0032                               brif v40, block5(v42), block4
;;
;;                                 block4 cold:
;; @0032                               v44 = iconst.i32 0
;; @0032                               v46 = uextend.i64 v2
;; @0032                               v47 = call fn0(v0, v44, v46)  ; v44 = 0
;; @0032                               jump block5(v47)
;;
;;                                 block5(v43: i64):
;; @0032                               v49 = load.i64 notrap aligned readonly can_move v0+40
;; @0032                               v50 = load.i32 notrap aligned readonly can_move v49
;; @0032                               v51 = load.i32 user7 aligned readonly v43+16
;; @0032                               v52 = icmp eq v51, v50
;; @0032                               trapz v52, user8
;; @0032                               v53 = load.i64 notrap aligned readonly v43+8
;; @0032                               v54 = load.i64 notrap aligned readonly v43+24
;; @0032                               v55 = call_indirect sig0, v53(v54, v0)
;; @0035                               jump block1
;;
;;                                 block1:
;; @0035                               return v29, v55
;; }
