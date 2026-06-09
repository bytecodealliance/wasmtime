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
;;                                     v90 = iconst.i64 3
;;                                     v91 = ishl v6, v90  ; v90 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v91, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 24
;;                                     v97 = iconst.i32 3
;;                                     v98 = ishl v3, v97  ; v97 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v98, user18  ; v5 = 24
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
;;                                     v106 = iconst.i32 15
;;                                     v107 = iadd.i32 v12, v106  ; v106 = 15
;;                                     v110 = iconst.i32 -16
;;                                     v111 = band v107, v110  ; v110 = -16
;;                                     v113 = iadd.i32 v14, v111
;; @0022                               store notrap aligned region0 v113, v13
;;                                     v129 = iconst.i32 -1476395002
;;                                     v130 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v131 = load.i64 notrap aligned readonly can_move v130+32
;; @0022                               v38 = iadd v131, v21
;; @0022                               store notrap aligned v129, v38  ; v129 = -1476395002
;;                                     v132 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v133 = load.i32 notrap aligned readonly can_move v132
;; @0022                               store notrap aligned v133, v38+4
;;                                     v134 = band.i64 v19, v18  ; v18 = -16
;; @0022                               istore32 notrap aligned v134, v38+8
;; @0022                               jump block4(v14, v38)
;;
;;                                 block3 cold:
;; @0022                               v25 = iconst.i32 -1476395002
;; @0022                               v26 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v27 = load.i32 notrap aligned readonly can_move v26
;; @0022                               v28 = iconst.i32 16
;; @0022                               v29 = call fn0(v0, v25, v27, v12, v28)  ; v25 = -1476395002, v28 = 16
;; @0022                               v30 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v31 = load.i64 notrap aligned readonly can_move v30+32
;; @0022                               v32 = uextend.i64 v29
;; @0022                               v33 = iadd v31, v32
;; @0022                               jump block4(v29, v33)
;;
;;                                 block4(v42: i32, v43: i64):
;; @0022                               v44 = iconst.i64 16
;; @0022                               v45 = iadd v43, v44  ; v44 = 16
;; @0022                               store.i32 user2 region1 v3, v45
;; @0022                               trapz v42, user16
;;                                     v135 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v136 = load.i64 notrap aligned readonly can_move v135+32
;; @0022                               v47 = uextend.i64 v42
;; @0022                               v49 = iadd v136, v47
;; @0022                               v51 = iadd v49, v44  ; v44 = 16
;; @0022                               v52 = load.i32 user2 readonly region1 v51
;; @0022                               v53 = uextend.i64 v52
;; @0022                               v59 = icmp.i64 ugt v6, v53
;; @0022                               trapnz v59, user17
;; @0022                               v75 = load.i64 notrap aligned v135+40
;; @0022                               v64 = iconst.i64 24
;; @0022                               v65 = iadd v49, v64  ; v64 = 24
;; @0022                               v77 = uadd_overflow_trap v65, v91, user2
;; @0022                               v76 = iadd v136, v75
;; @0022                               v78 = icmp ugt v77, v76
;; @0022                               trapnz v78, user2
;;                                     v115 = iconst.i64 0
;; @0022                               v81 = icmp.i64 eq v6, v115  ; v115 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v79 = iadd v65, v91
;; @0022                               brif v81, block6, block5(v65)
;;
;;                                 block5(v82: i64):
;; @0022                               store.i64 user2 little region1 v2, v82
;;                                     v137 = iconst.i64 8
;;                                     v138 = iadd v82, v137  ; v137 = 8
;; @0022                               v85 = icmp eq v138, v79
;; @0022                               brif v85, block6, block5(v138)
;;
;;                                 block6:
;; @0025                               jump block1(v42)
;;
;;                                 block1(v4: i32):
;; @0025                               return v4
;; }
