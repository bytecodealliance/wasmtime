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
;;                                     v97 = iconst.i64 32
;; @0022                               v9 = ushr v99, v97  ; v97 = 32
;; @0022                               trapnz v9, user18
;; @0022                               v5 = iconst.i32 24
;;                                     v105 = iconst.i32 3
;;                                     v106 = ishl v3, v105  ; v105 = 3
;; @0022                               v11 = uadd_overflow_trap v5, v106, user18  ; v5 = 24
;; @0022                               v13 = load.i64 notrap aligned readonly can_move v0+32
;; @0022                               v14 = load.i32 notrap aligned v13
;; @0022                               v15 = load.i32 notrap aligned v13+4
;; @0022                               v21 = uextend.i64 v14
;; @0022                               v16 = uextend.i64 v11
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
;;                                     v114 = iconst.i32 15
;;                                     v115 = iadd.i32 v11, v114  ; v114 = 15
;;                                     v118 = iconst.i32 -16
;;                                     v119 = band v115, v118  ; v118 = -16
;;                                     v121 = iadd.i32 v14, v119
;; @0022                               store notrap aligned region0 v121, v13
;;                                     v137 = iconst.i32 -1476395002
;;                                     v138 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v139 = load.i64 notrap aligned readonly can_move v138+32
;; @0022                               v38 = iadd v139, v21
;; @0022                               store notrap aligned v137, v38  ; v137 = -1476395002
;;                                     v140 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v141 = load.i32 notrap aligned readonly can_move v140
;; @0022                               store notrap aligned v141, v38+4
;;                                     v142 = band.i64 v19, v18  ; v18 = -16
;; @0022                               istore32 notrap aligned v142, v38+8
;; @0022                               jump block4(v14, v38)
;;
;;                                 block3 cold:
;; @0022                               v26 = iconst.i32 -1476395002
;; @0022                               v28 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v29 = load.i32 notrap aligned readonly can_move v28
;; @0022                               v30 = iconst.i32 16
;; @0022                               v31 = call fn0(v0, v26, v29, v11, v30)  ; v26 = -1476395002, v30 = 16
;; @0022                               v93 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v32 = load.i64 notrap aligned readonly can_move v93+32
;; @0022                               v33 = uextend.i64 v31
;; @0022                               v34 = iadd v32, v33
;; @0022                               jump block4(v31, v34)
;;
;;                                 block4(v43: i32, v44: i64):
;; @0022                               v45 = iconst.i64 16
;; @0022                               v46 = iadd v44, v45  ; v45 = 16
;; @0022                               store.i32 user2 region1 v3, v46
;; @0022                               trapz v43, user16
;;                                     v143 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v144 = load.i64 notrap aligned readonly can_move v143+32
;; @0022                               v48 = uextend.i64 v43
;; @0022                               v50 = iadd v144, v48
;; @0022                               v52 = iadd v50, v45  ; v45 = 16
;; @0022                               v53 = load.i32 user2 readonly region1 v52
;; @0022                               v54 = uextend.i64 v53
;; @0022                               v60 = icmp.i64 ugt v6, v54
;; @0022                               trapnz v60, user17
;; @0022                               v74 = load.i64 notrap aligned v143+40
;; @0022                               v64 = iconst.i64 24
;; @0022                               v65 = iadd v50, v64  ; v64 = 24
;; @0022                               v76 = uadd_overflow_trap v65, v99, user2
;; @0022                               v75 = iadd v144, v74
;; @0022                               v77 = icmp ugt v76, v75
;; @0022                               trapnz v77, user2
;;                                     v123 = iconst.i64 0
;; @0022                               v80 = icmp.i64 eq v6, v123  ; v123 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v78 = iadd v65, v99
;; @0022                               brif v80, block6, block5(v65)
;;
;;                                 block5(v81: i64):
;; @0022                               store.i64 user2 little region1 v2, v81
;;                                     v145 = iconst.i64 8
;;                                     v146 = iadd v81, v145  ; v145 = 8
;; @0022                               v84 = icmp eq v146, v78
;; @0022                               brif v84, block6, block5(v146)
;;
;;                                 block6:
;; @0025                               jump block1(v43)
;;
;;                                 block1(v4: i32):
;; @0025                               return v4
;; }
