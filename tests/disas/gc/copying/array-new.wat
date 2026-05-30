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
;;                                     v98 = iconst.i64 3
;;                                     v99 = ishl v6, v98  ; v98 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v99, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 24
;;                                     v105 = iconst.i32 3
;;                                     v106 = ishl v3, v105  ; v105 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v106, user18  ; v5 = 24
;; @0022                               v14 = load.i64 notrap aligned readonly can_move v0+32
;; @0022                               v15 = load.i32 notrap aligned v14
;; @0022                               v16 = load.i32 notrap aligned v14+4
;; @0022                               v22 = uextend.i64 v15
;; @0022                               v17 = uextend.i64 v12
;; @0022                               v18 = iconst.i64 15
;; @0022                               v20 = iadd v17, v18  ; v18 = 15
;; @0022                               v19 = iconst.i64 -16
;; @0022                               v21 = band v20, v19  ; v19 = -16
;; @0022                               v23 = iadd v22, v21
;; @0022                               v24 = uextend.i64 v16
;; @0022                               v25 = icmp ule v23, v24
;; @0022                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v114 = iconst.i32 15
;;                                     v115 = iadd.i32 v12, v114  ; v114 = 15
;;                                     v118 = iconst.i32 -16
;;                                     v119 = band v115, v118  ; v118 = -16
;;                                     v121 = iadd.i32 v15, v119
;; @0022                               store notrap aligned region0 v121, v14
;;                                     v137 = iconst.i32 -1476395002
;;                                     v138 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v139 = load.i64 notrap aligned readonly can_move v138+32
;; @0022                               v39 = iadd v139, v22
;; @0022                               store notrap aligned v137, v39  ; v137 = -1476395002
;;                                     v140 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v141 = load.i32 notrap aligned readonly can_move v140
;; @0022                               store notrap aligned v141, v39+4
;;                                     v142 = band.i64 v20, v19  ; v19 = -16
;; @0022                               istore32 notrap aligned v142, v39+8
;; @0022                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0022                               v27 = iconst.i32 -1476395002
;; @0022                               v29 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v30 = load.i32 notrap aligned readonly can_move v29
;; @0022                               v31 = iconst.i32 16
;; @0022                               v32 = call fn0(v0, v27, v30, v12, v31)  ; v27 = -1476395002, v31 = 16
;; @0022                               v94 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v33 = load.i64 notrap aligned readonly can_move v94+32
;; @0022                               v34 = uextend.i64 v32
;; @0022                               v35 = iadd v33, v34
;; @0022                               jump block4(v32, v35)
;;
;;                                 block4(v44: i32, v45: i64):
;; @0022                               v46 = iconst.i64 16
;; @0022                               v47 = iadd v45, v46  ; v46 = 16
;; @0022                               store.i32 user2 region1 v3, v47
;; @0022                               trapz v44, user16
;;                                     v143 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v144 = load.i64 notrap aligned readonly can_move v143+32
;; @0022                               v49 = uextend.i64 v44
;; @0022                               v51 = iadd v144, v49
;; @0022                               v53 = iadd v51, v46  ; v46 = 16
;; @0022                               v54 = load.i32 user2 readonly region1 v53
;; @0022                               v55 = uextend.i64 v54
;; @0022                               v61 = icmp.i64 ugt v6, v55
;; @0022                               trapnz v61, user17
;; @0022                               v75 = load.i64 notrap aligned v143+40
;; @0022                               v65 = iconst.i64 24
;; @0022                               v66 = iadd v51, v65  ; v65 = 24
;; @0022                               v77 = uadd_overflow_trap v66, v99, user2
;; @0022                               v76 = iadd v144, v75
;; @0022                               v78 = icmp ugt v77, v76
;; @0022                               trapnz v78, user2
;;                                     v123 = iconst.i64 0
;; @0022                               v81 = icmp.i64 eq v6, v123  ; v123 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v79 = iadd v66, v99
;; @0022                               brif v81, block6, block5(v66)
;;
;;                                 block5(v82: i64):
;; @0022                               store.i64 user2 little region1 v2, v82
;;                                     v145 = iconst.i64 8
;;                                     v146 = iadd v82, v145  ; v145 = 8
;; @0022                               v85 = icmp eq v146, v79
;; @0022                               brif v85, block6, block5(v146)
;;
;;                                 block6:
;; @0025                               jump block1(v44)
;;
;;                                 block1(v4: i32):
;; @0025                               return v4
;; }
