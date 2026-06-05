;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut funcref)))

  (func $fill (param $len i32) (result (ref $a))
    (array.new_default $a (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
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
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               v5 = uextend.i64 v2
;;                                     v110 = iconst.i64 2
;;                                     v111 = ishl v5, v110  ; v110 = 2
;; @001f                               v8 = iconst.i64 32
;; @001f                               v9 = ushr v111, v8  ; v8 = 32
;; @001f                               trapnz v9, user18
;; @001f                               v4 = iconst.i32 20
;;                                     v117 = iconst.i32 2
;;                                     v118 = ishl v2, v117  ; v117 = 2
;; @001f                               v11 = uadd_overflow_trap v4, v118, user18  ; v4 = 20
;; @001f                               v13 = load.i64 notrap aligned readonly can_move v0+32
;; @001f                               v14 = load.i32 notrap aligned v13
;; @001f                               v15 = load.i32 notrap aligned v13+4
;; @001f                               v21 = uextend.i64 v14
;; @001f                               v16 = uextend.i64 v11
;; @001f                               v17 = iconst.i64 15
;; @001f                               v19 = iadd v16, v17  ; v17 = 15
;; @001f                               v18 = iconst.i64 -16
;; @001f                               v20 = band v19, v18  ; v18 = -16
;; @001f                               v22 = iadd v21, v20
;; @001f                               v23 = uextend.i64 v15
;; @001f                               v24 = icmp ule v22, v23
;; @001f                               brif v24, block2, block3
;;
;;                                 block2:
;;                                     v126 = iconst.i32 15
;;                                     v127 = iadd.i32 v11, v126  ; v126 = 15
;;                                     v130 = iconst.i32 -16
;;                                     v131 = band v127, v130  ; v130 = -16
;;                                     v133 = iadd.i32 v14, v131
;; @001f                               store notrap aligned region0 v133, v13
;;                                     v148 = iconst.i32 -1476395002
;;                                     v149 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v150 = load.i64 notrap aligned readonly can_move v149+32
;; @001f                               v38 = iadd v150, v21
;; @001f                               store notrap aligned v148, v38  ; v148 = -1476395002
;;                                     v151 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v152 = load.i32 notrap aligned readonly can_move v151
;; @001f                               store notrap aligned v152, v38+4
;;                                     v153 = band.i64 v19, v18  ; v18 = -16
;; @001f                               istore32 notrap aligned v153, v38+8
;; @001f                               jump block4(v14, v38)
;;
;;                                 block3 cold:
;; @001f                               v26 = iconst.i32 -1476395002
;; @001f                               v28 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v29 = load.i32 notrap aligned readonly can_move v28
;; @001f                               v30 = iconst.i32 16
;; @001f                               v31 = call fn0(v0, v26, v29, v11, v30)  ; v26 = -1476395002, v30 = 16
;; @001f                               v106 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v32 = load.i64 notrap aligned readonly can_move v106+32
;; @001f                               v33 = uextend.i64 v31
;; @001f                               v34 = iadd v32, v33
;; @001f                               jump block4(v31, v34)
;;
;;                                 block4(v43: i32, v44: i64):
;;                                     v97 = stack_addr.i64 ss0
;;                                     store notrap v43, v97
;; @001f                               v45 = iconst.i64 16
;; @001f                               v46 = iadd v44, v45  ; v45 = 16
;; @001f                               store.i32 user2 region1 v2, v46
;; @001f                               trapz v43, user16
;;                                     v154 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v155 = load.i64 notrap aligned readonly can_move v154+32
;; @001f                               v49 = uextend.i64 v43
;; @001f                               v51 = iadd v155, v49
;; @001f                               v53 = iadd v51, v45  ; v45 = 16
;; @001f                               v54 = load.i32 user2 readonly region1 v53
;; @001f                               v55 = uextend.i64 v54
;; @001f                               v61 = icmp.i64 ugt v5, v55
;; @001f                               trapnz v61, user17
;; @001f                               v75 = load.i64 notrap aligned v154+40
;; @001f                               v65 = iconst.i64 20
;; @001f                               v66 = iadd v51, v65  ; v65 = 20
;; @001f                               v77 = uadd_overflow_trap v66, v111, user2
;; @001f                               v76 = iadd v155, v75
;; @001f                               v78 = icmp ugt v77, v76
;; @001f                               trapnz v78, user2
;; @001f                               v47 = iconst.i64 0
;; @001f                               v80 = call fn1(v0, v47), stack_map=[i32 @ ss0+0]  ; v47 = 0
;; @001f                               v84 = icmp.i64 eq v5, v47  ; v47 = 0
;; @001f                               v81 = ireduce.i32 v80
;; @001f                               v6 = iconst.i64 4
;; @001f                               v82 = iadd v66, v111
;; @001f                               brif v84, block6, block5(v66)
;;
;;                                 block5(v85: i64):
;; @001f                               store.i32 notrap aligned little v81, v85
;;                                     v156 = iconst.i64 4
;;                                     v157 = iadd v85, v156  ; v156 = 4
;; @001f                               v88 = icmp eq v157, v82
;; @001f                               brif v88, block6, block5(v157)
;;
;;                                 block6:
;;                                     v90 = load.i32 notrap v97
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v90
;; }
