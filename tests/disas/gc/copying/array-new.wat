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
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
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
;;                                     v94 = iconst.i64 3
;;                                     v95 = ishl v6, v94  ; v94 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v95, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 24
;;                                     v101 = iconst.i32 3
;;                                     v102 = ishl v3, v101  ; v101 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v102, user18  ; v5 = 24
;; @0022                               v13 = load.i64 notrap aligned readonly can_move v0+32
;; @0022                               v14 = load.i32 notrap aligned v13
;; @0022                               v15 = load.i32 notrap aligned v13+4
;; @0022                               v21 = uextend.i64 v14
;; @0022                               v16 = uextend.i64 v12
;; @0022                               v17 = iconst.i64 15
;; @0022                               v19 = iadd v16, v17  ; v17 = 15
;; @0022                               v18 = iconst.i64 -16
;; @0022                               v20 = band v19, v18  ; v18 = -16
;; @0022                               v22 = iadd v21, v20
;; @0022                               v23 = uextend.i64 v15
;; @0022                               v24 = icmp ule v22, v23
;; @0022                               brif v24, block2, block3
;;
;;                                 block2:
;;                                     v110 = iconst.i32 15
;;                                     v111 = iadd.i32 v12, v110  ; v110 = 15
;;                                     v114 = iconst.i32 -16
;;                                     v115 = band v111, v114  ; v114 = -16
;;                                     v117 = iadd.i32 v14, v115
;; @0022                               store notrap aligned region0 v117, v13
;;                                     v133 = iconst.i32 -1476395002
;;                                     v134 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v135 = load.i64 notrap aligned readonly can_move v134+32
;; @0022                               v36 = iadd v135, v21
;; @0022                               store notrap aligned v133, v36  ; v133 = -1476395002
;;                                     v136 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v137 = load.i32 notrap aligned readonly can_move v136
;; @0022                               store notrap aligned v137, v36+4
;;                                     v138 = band.i64 v19, v18  ; v18 = -16
;; @0022                               istore32 notrap aligned v138, v36+8
;; @0022                               jump block4(v14, v36)
;;
;;                                 block3 cold:
;; @0022                               v25 = iconst.i32 -1476395002
;; @0022                               v26 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v27 = load.i32 notrap aligned readonly can_move v26
;; @0022                               v28 = iconst.i32 16
;; @0022                               v29 = call fn0(v0, v25, v27, v12, v28)  ; v25 = -1476395002, v28 = 16
;; @0022                               v90 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v30 = load.i64 notrap aligned readonly can_move v90+32
;; @0022                               v31 = uextend.i64 v29
;; @0022                               v32 = iadd v30, v31
;; @0022                               jump block4(v29, v32)
;;
;;                                 block4(v40: i32, v41: i64):
;; @0022                               v42 = iconst.i64 16
;; @0022                               v43 = iadd v41, v42  ; v42 = 16
;; @0022                               store.i32 user2 region1 v3, v43
;; @0022                               trapz v40, user16
;;                                     v139 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v140 = load.i64 notrap aligned readonly can_move v139+32
;; @0022                               v45 = uextend.i64 v40
;; @0022                               v47 = iadd v140, v45
;; @0022                               v49 = iadd v47, v42  ; v42 = 16
;; @0022                               v50 = load.i32 user2 readonly region1 v49
;; @0022                               v51 = uextend.i64 v50
;; @0022                               v57 = icmp.i64 ugt v6, v51
;; @0022                               trapnz v57, user17
;; @0022                               v71 = load.i64 notrap aligned v139+40
;; @0022                               v61 = iconst.i64 24
;; @0022                               v62 = iadd v47, v61  ; v61 = 24
;; @0022                               v73 = uadd_overflow_trap v62, v95, user2
;; @0022                               v72 = iadd v140, v71
;; @0022                               v74 = icmp ugt v73, v72
;; @0022                               trapnz v74, user2
;;                                     v119 = iconst.i64 0
;; @0022                               v77 = icmp.i64 eq v6, v119  ; v119 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v75 = iadd v62, v95
;; @0022                               brif v77, block6, block5(v62)
;;
;;                                 block5(v78: i64):
;; @0022                               store.i64 user2 little region1 v2, v78
;;                                     v141 = iconst.i64 8
;;                                     v142 = iadd v78, v141  ; v141 = 8
;; @0022                               v81 = icmp eq v142, v75
;; @0022                               brif v81, block6, block5(v142)
;;
;;                                 block6:
;; @0025                               jump block1(v40)
;;
;;                                 block1(v4: i32):
;; @0025                               return v4
;; }
