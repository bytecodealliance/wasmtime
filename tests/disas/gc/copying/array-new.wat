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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 32 "VMContext+0x20"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v88 = iconst.i64 3
;;                                     v89 = ishl v6, v88  ; v88 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v89, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 24
;;                                     v95 = iconst.i32 3
;;                                     v96 = ishl v3, v95  ; v95 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v96, user18  ; v5 = 24
;; @0022                               v13 = load.i64 notrap aligned readonly can_move region1 v0+32
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
;;                                     v104 = iconst.i32 15
;;                                     v105 = iadd.i32 v12, v104  ; v104 = 15
;;                                     v108 = iconst.i32 -16
;;                                     v109 = band v105, v108  ; v108 = -16
;;                                     v111 = iadd.i32 v14, v109
;; @0022                               store notrap aligned v111, v13
;;                                     v127 = iconst.i32 -1476395002
;;                                     v128 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v129 = load.i64 notrap aligned readonly can_move v128+32
;; @0022                               v38 = iadd v129, v21
;; @0022                               store notrap aligned v127, v38  ; v127 = -1476395002
;;                                     v130 = load.i64 notrap aligned readonly can_move region2 v0+40
;;                                     v131 = load.i32 notrap aligned readonly can_move v130
;; @0022                               store notrap aligned v131, v38+4
;;                                     v132 = band.i64 v19, v18  ; v18 = -16
;; @0022                               istore32 notrap aligned v132, v38+8
;; @0022                               jump block4(v14, v38)
;;
;;                                 block3 cold:
;; @0022                               v25 = iconst.i32 -1476395002
;; @0022                               v26 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0022                               v27 = load.i32 notrap aligned readonly can_move v26
;; @0022                               v28 = iconst.i32 16
;; @0022                               v29 = call fn0(v0, v25, v27, v12, v28)  ; v25 = -1476395002, v28 = 16
;; @0022                               v30 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v31 = load.i64 notrap aligned readonly can_move v30+32
;; @0022                               v32 = uextend.i64 v29
;; @0022                               v33 = iadd v31, v32
;; @0022                               jump block4(v29, v33)
;;
;;                                 block4(v42: i32, v43: i64):
;; @0022                               v44 = iconst.i64 16
;; @0022                               v45 = iadd v43, v44  ; v44 = 16
;; @0022                               store.i32 user2 region3 v3, v45
;; @0022                               trapz v42, user16
;;                                     v133 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v134 = load.i64 notrap aligned readonly can_move v133+32
;; @0022                               v47 = uextend.i64 v42
;; @0022                               v50 = iadd v134, v47
;; @0022                               v52 = iadd v50, v44  ; v44 = 16
;; @0022                               v53 = load.i32 user2 readonly region3 v52
;; @0022                               v54 = uextend.i64 v53
;; @0022                               v60 = icmp.i64 ugt v6, v54
;; @0022                               trapnz v60, user17
;; @0022                               v77 = load.i64 notrap aligned v133+40
;; @0022                               v65 = iconst.i64 24
;; @0022                               v66 = iadd v50, v65  ; v65 = 24
;; @0022                               v79 = uadd_overflow_trap v66, v89, user2
;; @0022                               v78 = iadd v134, v77
;; @0022                               v80 = icmp ugt v79, v78
;; @0022                               trapnz v80, user2
;;                                     v113 = iconst.i64 0
;; @0022                               v83 = icmp.i64 eq v6, v113  ; v113 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v81 = iadd v66, v89
;; @0022                               brif v83, block6, block5(v66)
;;
;;                                 block5(v84: i64):
;; @0022                               store.i64 user2 little region3 v2, v84
;;                                     v135 = iconst.i64 8
;;                                     v136 = iadd v84, v135  ; v135 = 8
;; @0022                               v87 = icmp eq v136, v81
;; @0022                               brif v87, block6, block5(v136)
;;
;;                                 block6:
;; @0025                               jump block1(v42)
;;
;;                                 block1(v4: i32):
;; @0025                               return v4
;; }
