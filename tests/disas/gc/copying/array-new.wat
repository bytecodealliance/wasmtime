;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param i64 i32) (result (ref $ty))
    (array.new $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v62 = iconst.i64 3
;;                                     v63 = ishl v6, v62  ; v62 = 3
;;                                     v60 = iconst.i64 32
;; @0022                               v8 = ushr v63, v60  ; v60 = 32
;; @0022                               trapnz v8, user17
;; @0022                               v5 = iconst.i32 24
;;                                     v69 = iconst.i32 3
;;                                     v70 = ishl v3, v69  ; v69 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v70, user17  ; v5 = 24
;; @0022                               v12 = load.i64 notrap aligned readonly can_move v0+32
;; @0022                               v13 = load.i32 notrap aligned can_move v12
;; @0022                               v20 = uextend.i64 v13
;; @0022                               v15 = uextend.i64 v10
;; @0022                               v16 = iconst.i64 15
;; @0022                               v18 = iadd v15, v16  ; v16 = 15
;; @0022                               v17 = iconst.i64 -16
;; @0022                               v19 = band v18, v17  ; v17 = -16
;; @0022                               v21 = iadd v20, v19
;; @0022                               v14 = load.i32 notrap aligned readonly can_move v12+4
;; @0022                               v22 = uextend.i64 v14
;; @0022                               v23 = icmp ule v21, v22
;; @0022                               brif v23, block2, block3
;;
;;                                 block2:
;;                                     v78 = iconst.i32 15
;;                                     v79 = iadd.i32 v10, v78  ; v78 = 15
;;                                     v82 = iconst.i32 -16
;;                                     v83 = band v79, v82  ; v82 = -16
;;                                     v85 = iadd.i32 v13, v83
;; @0022                               store notrap aligned vmctx v85, v12
;;                                     v98 = iconst.i32 -1476395008
;;                                     v99 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v100 = load.i64 notrap aligned readonly can_move v99+32
;; @0022                               v37 = iadd v100, v20
;; @0022                               store notrap aligned v98, v37  ; v98 = -1476395008
;;                                     v101 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v102 = load.i32 notrap aligned readonly can_move v101
;; @0022                               store notrap aligned v102, v37+4
;;                                     v103 = band.i64 v18, v17  ; v17 = -16
;; @0022                               istore32 notrap aligned v103, v37+8
;; @0022                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @0022                               v25 = iconst.i32 -1476395008
;; @0022                               v27 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v28 = load.i32 notrap aligned readonly can_move v27
;; @0022                               v29 = iconst.i32 16
;; @0022                               v30 = call fn0(v0, v25, v28, v10, v29)  ; v25 = -1476395008, v29 = 16
;; @0022                               v56 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v31 = load.i64 notrap aligned readonly can_move v56+32
;; @0022                               v32 = uextend.i64 v30
;; @0022                               v33 = iadd v31, v32
;; @0022                               jump block4(v30, v33)
;;
;;                                 block4(v42: i32, v43: i64):
;;                                     v55 = iconst.i64 16
;; @0022                               v44 = iadd v43, v55  ; v55 = 16
;; @0022                               store.i32 notrap aligned v3, v44
;;                                     v88 = iconst.i64 24
;;                                     v93 = iadd v43, v88  ; v88 = 24
;; @0022                               v51 = iadd v43, v15
;;                                     v61 = iconst.i64 8
;; @0022                               jump block5(v93)
;;
;;                                 block5(v52: i64):
;; @0022                               v53 = icmp eq v52, v51
;; @0022                               brif v53, block7, block6
;;
;;                                 block6:
;; @0022                               store.i64 notrap aligned little v2, v52
;;                                     v104 = iconst.i64 8
;;                                     v105 = iadd.i64 v52, v104  ; v104 = 8
;; @0022                               jump block5(v105)
;;
;;                                 block7:
;; @0025                               jump block1(v42)
;;
;;                                 block1(v4: i32):
;; @0025                               return v4
;; }
