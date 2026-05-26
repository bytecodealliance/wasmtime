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
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v98 = iconst.i64 3
;;                                     v99 = ishl v6, v98  ; v98 = 3
;;                                     v96 = iconst.i64 32
;; @0022                               v8 = ushr v99, v96  ; v96 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 24
;;                                     v105 = iconst.i32 3
;;                                     v106 = ishl v3, v105  ; v105 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v106, user18  ; v5 = 24
;; @0022                               v12 = load.i64 notrap aligned readonly can_move v0+32
;; @0022                               v13 = load.i32 notrap aligned v12
;; @0022                               v14 = load.i32 notrap aligned v12+4
;; @0022                               v20 = uextend.i64 v13
;; @0022                               v15 = uextend.i64 v10
;; @0022                               v16 = iconst.i64 15
;; @0022                               v18 = iadd v15, v16  ; v16 = 15
;; @0022                               v17 = iconst.i64 -16
;; @0022                               v19 = band v18, v17  ; v17 = -16
;; @0022                               v21 = iadd v20, v19
;; @0022                               v22 = uextend.i64 v14
;; @0022                               v23 = icmp ule v21, v22
;; @0022                               brif v23, block2, block3
;;
;;                                 block2:
;;                                     v114 = iconst.i32 15
;;                                     v115 = iadd.i32 v10, v114  ; v114 = 15
;;                                     v118 = iconst.i32 -16
;;                                     v119 = band v115, v118  ; v118 = -16
;;                                     v121 = iadd.i32 v13, v119
;; @0022                               store notrap aligned vmctx v121, v12
;;                                     v137 = iconst.i32 -1476395008
;;                                     v138 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v139 = load.i64 notrap aligned readonly can_move v138+32
;; @0022                               v37 = iadd v139, v20
;; @0022                               store notrap aligned v137, v37  ; v137 = -1476395008
;;                                     v140 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v141 = load.i32 notrap aligned readonly can_move v140
;; @0022                               store notrap aligned v141, v37+4
;;                                     v142 = band.i64 v18, v17  ; v17 = -16
;; @0022                               istore32 notrap aligned v142, v37+8
;; @0022                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @0022                               v25 = iconst.i32 -1476395008
;; @0022                               v27 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v28 = load.i32 notrap aligned readonly can_move v27
;; @0022                               v29 = iconst.i32 16
;; @0022                               v30 = call fn0(v0, v25, v28, v10, v29)  ; v25 = -1476395008, v29 = 16
;; @0022                               v92 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v31 = load.i64 notrap aligned readonly can_move v92+32
;; @0022                               v32 = uextend.i64 v30
;; @0022                               v33 = iadd v31, v32
;; @0022                               jump block4(v30, v33)
;;
;;                                 block4(v42: i32, v43: i64):
;;                                     v91 = iconst.i64 16
;; @0022                               v44 = iadd v43, v91  ; v91 = 16
;; @0022                               store.i32 user2 v3, v44
;; @0022                               trapz v42, user16
;;                                     v143 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v144 = load.i64 notrap aligned readonly can_move v143+32
;; @0022                               v46 = uextend.i64 v42
;; @0022                               v48 = iadd v144, v46
;; @0022                               v50 = iadd v48, v91  ; v91 = 16
;; @0022                               v51 = load.i32 user2 readonly v50
;; @0022                               v52 = uextend.i64 v51
;; @0022                               v57 = icmp.i64 ugt v6, v52
;; @0022                               trapnz v57, user17
;; @0022                               v68 = load.i64 notrap aligned v143+40
;;                                     v85 = iconst.i64 24
;; @0022                               v61 = iadd v48, v85  ; v85 = 24
;; @0022                               v70 = uadd_overflow_trap v61, v99, user2
;; @0022                               v69 = iadd v144, v68
;; @0022                               v71 = icmp ugt v70, v69
;; @0022                               trapnz v71, user2
;;                                     v123 = iconst.i64 0
;; @0022                               v73 = icmp.i64 eq v6, v123  ; v123 = 0
;;                                     v97 = iconst.i64 8
;; @0022                               v72 = iadd v61, v99
;; @0022                               brif v73, block6, block5(v61)
;;
;;                                 block5(v74: i64):
;; @0022                               store.i64 user2 little v2, v74
;;                                     v145 = iconst.i64 8
;;                                     v146 = iadd v74, v145  ; v145 = 8
;; @0022                               v76 = icmp eq v146, v72
;; @0022                               brif v76, block6, block5(v146)
;;
;;                                 block6:
;; @0025                               jump block1(v42)
;;
;;                                 block1(v4: i32):
;; @0025                               return v4
;; }
