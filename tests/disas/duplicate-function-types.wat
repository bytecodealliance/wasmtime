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
;; @002d                               v3 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @002d                               v4 = load.i64 notrap aligned region4 v3+8
;; @002d                               v5 = ireduce.i32 v4
;; @002d                               v6 = icmp uge v2, v5
;; @002d                               v7 = uextend.i64 v2
;; @002d                               v8 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @002d                               v9 = load.i64 notrap aligned region3 v8
;; @002d                               v10 = iconst.i64 3
;; @002d                               v11 = ishl v7, v10  ; v10 = 3
;; @002d                               v12 = iadd v9, v11
;; @002d                               v13 = iconst.i64 0
;; @002d                               v14 = select_spectre_guard v6, v13, v12  ; v13 = 0
;; @002d                               v15 = load.i64 user6 aligned region5 v14
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
;; @002d                               v22 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @002d                               v23 = load.i32 notrap aligned readonly can_move v22
;; @002d                               v24 = load.i32 user7 aligned readonly v18+16
;; @002d                               v25 = icmp eq v24, v23
;; @002d                               v26 = uextend.i32 v25
;; @002d                               trapz v26, user8
;; @002d                               v27 = load.i64 notrap aligned readonly v18+8
;; @002d                               v28 = load.i64 notrap aligned readonly v18+24
;; @002d                               v29 = call_indirect sig0, v27(v28, v0)
;; @0032                               v31 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0032                               v32 = load.i64 notrap aligned region4 v31+8
;; @0032                               v33 = ireduce.i32 v32
;; @0032                               v34 = icmp.i32 uge v2, v33
;; @0032                               v35 = uextend.i64 v2
;; @0032                               v36 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0032                               v37 = load.i64 notrap aligned region3 v36
;; @0032                               v38 = iconst.i64 3
;; @0032                               v39 = ishl v35, v38  ; v38 = 3
;; @0032                               v40 = iadd v37, v39
;; @0032                               v41 = iconst.i64 0
;; @0032                               v42 = select_spectre_guard v34, v41, v40  ; v41 = 0
;; @0032                               v43 = load.i64 user6 aligned region5 v42
;; @0032                               v44 = iconst.i64 -2
;; @0032                               v45 = band v43, v44  ; v44 = -2
;; @0032                               brif v43, block5(v45), block4
;;
;;                                 block4 cold:
;; @0032                               v47 = iconst.i32 0
;; @0032                               v48 = uextend.i64 v2
;; @0032                               v49 = call fn0(v0, v47, v48)  ; v47 = 0
;; @0032                               jump block5(v49)
;;
;;                                 block5(v46: i64):
;; @0032                               v50 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0032                               v51 = load.i32 notrap aligned readonly can_move v50
;; @0032                               v52 = load.i32 user7 aligned readonly v46+16
;; @0032                               v53 = icmp eq v52, v51
;; @0032                               v54 = uextend.i32 v53
;; @0032                               trapz v54, user8
;; @0032                               v55 = load.i64 notrap aligned readonly v46+8
;; @0032                               v56 = load.i64 notrap aligned readonly v46+24
;; @0032                               v57 = call_indirect sig0, v55(v56, v0)
;; @0035                               jump block1
;;
;;                                 block1:
;; @0035                               return v29, v57
;; }
