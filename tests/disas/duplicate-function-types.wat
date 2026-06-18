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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 48 "VMContext+0x30"
;;     region3 = 2684354560 "VMTableDefinition+0x0"
;;     region4 = 2684354568 "VMTableDefinition+0x8"
;;     region5 = 1073741824 "PublicTable"
;;     region6 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002d                               v5 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @002d                               v6 = load.i64 notrap aligned region4 v5+8
;; @002d                               v7 = ireduce.i32 v6
;; @002d                               v8 = icmp uge v2, v7
;; @002d                               v9 = uextend.i64 v2
;; @002d                               v10 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @002d                               v11 = load.i64 notrap aligned region3 v10
;; @002d                               v12 = iconst.i64 3
;; @002d                               v13 = ishl v9, v12  ; v12 = 3
;; @002d                               v14 = iadd v11, v13
;; @002d                               v15 = iconst.i64 0
;; @002d                               v16 = select_spectre_guard v8, v15, v14  ; v15 = 0
;; @002d                               v17 = load.i64 user6 aligned region5 v16
;; @002d                               v18 = iconst.i64 -2
;; @002d                               v19 = band v17, v18  ; v18 = -2
;; @002d                               brif v17, block3(v19), block2
;;
;;                                 block2 cold:
;; @002d                               v21 = iconst.i32 0
;; @002d                               v22 = uextend.i64 v2
;; @002d                               v23 = call fn0(v0, v21, v22)  ; v21 = 0
;; @002d                               jump block3(v23)
;;
;;                                 block3(v20: i64):
;; @002d                               v24 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @002d                               v25 = load.i32 notrap aligned readonly can_move v24
;; @002d                               v26 = load.i32 user7 aligned readonly v20+16
;; @002d                               v27 = icmp eq v26, v25
;; @002d                               v28 = uextend.i32 v27
;; @002d                               trapz v28, user8
;; @002d                               v29 = load.i64 notrap aligned readonly v20+8
;; @002d                               v30 = load.i64 notrap aligned readonly v20+24
;; @002d                               v31 = call_indirect sig0, v29(v30, v0)
;; @0032                               v33 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0032                               v34 = load.i64 notrap aligned region4 v33+8
;; @0032                               v35 = ireduce.i32 v34
;; @0032                               v36 = icmp.i32 uge v2, v35
;; @0032                               v37 = uextend.i64 v2
;; @0032                               v38 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0032                               v39 = load.i64 notrap aligned region3 v38
;; @0032                               v40 = iconst.i64 3
;; @0032                               v41 = ishl v37, v40  ; v40 = 3
;; @0032                               v42 = iadd v39, v41
;; @0032                               v43 = iconst.i64 0
;; @0032                               v44 = select_spectre_guard v36, v43, v42  ; v43 = 0
;; @0032                               v45 = load.i64 user6 aligned region5 v44
;; @0032                               v46 = iconst.i64 -2
;; @0032                               v47 = band v45, v46  ; v46 = -2
;; @0032                               brif v45, block5(v47), block4
;;
;;                                 block4 cold:
;; @0032                               v49 = iconst.i32 0
;; @0032                               v50 = uextend.i64 v2
;; @0032                               v51 = call fn0(v0, v49, v50)  ; v49 = 0
;; @0032                               jump block5(v51)
;;
;;                                 block5(v48: i64):
;; @0032                               v52 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0032                               v53 = load.i32 notrap aligned readonly can_move v52
;; @0032                               v54 = load.i32 user7 aligned readonly v48+16
;; @0032                               v55 = icmp eq v54, v53
;; @0032                               v56 = uextend.i32 v55
;; @0032                               trapz v56, user8
;; @0032                               v57 = load.i64 notrap aligned readonly v48+8
;; @0032                               v58 = load.i64 notrap aligned readonly v48+24
;; @0032                               v59 = call_indirect sig0, v57(v58, v0)
;; @0035                               jump block1
;;
;;                                 block1:
;; @0035                               return v31, v59
;; }
