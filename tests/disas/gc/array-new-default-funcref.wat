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
;;                                     v109 = iconst.i64 32
;; @001f                               v8 = ushr v111, v109  ; v109 = 32
;; @001f                               trapnz v8, user18
;; @001f                               v4 = iconst.i32 20
;;                                     v117 = iconst.i32 2
;;                                     v118 = ishl v2, v117  ; v117 = 2
;; @001f                               v10 = uadd_overflow_trap v4, v118, user18  ; v4 = 20
;; @001f                               v12 = load.i64 notrap aligned readonly can_move v0+32
;; @001f                               v13 = load.i32 notrap aligned v12
;; @001f                               v14 = load.i32 notrap aligned v12+4
;; @001f                               v20 = uextend.i64 v13
;; @001f                               v15 = uextend.i64 v10
;; @001f                               v16 = iconst.i64 15
;; @001f                               v18 = iadd v15, v16  ; v16 = 15
;; @001f                               v17 = iconst.i64 -16
;; @001f                               v19 = band v18, v17  ; v17 = -16
;; @001f                               v21 = iadd v20, v19
;; @001f                               v22 = uextend.i64 v14
;; @001f                               v23 = icmp ule v21, v22
;; @001f                               brif v23, block2, block3
;;
;;                                 block2:
;;                                     v126 = iconst.i32 15
;;                                     v127 = iadd.i32 v10, v126  ; v126 = 15
;;                                     v130 = iconst.i32 -16
;;                                     v131 = band v127, v130  ; v130 = -16
;;                                     v133 = iadd.i32 v13, v131
;; @001f                               store notrap aligned region0 v133, v12
;;                                     v148 = iconst.i32 -1476395002
;;                                     v149 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v150 = load.i64 notrap aligned readonly can_move v149+32
;; @001f                               v37 = iadd v150, v20
;; @001f                               store notrap aligned v148, v37  ; v148 = -1476395002
;;                                     v151 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v152 = load.i32 notrap aligned readonly can_move v151
;; @001f                               store notrap aligned v152, v37+4
;;                                     v153 = band.i64 v18, v17  ; v17 = -16
;; @001f                               istore32 notrap aligned v153, v37+8
;; @001f                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @001f                               v25 = iconst.i32 -1476395002
;; @001f                               v27 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v28 = load.i32 notrap aligned readonly can_move v27
;; @001f                               v29 = iconst.i32 16
;; @001f                               v30 = call fn0(v0, v25, v28, v10, v29)  ; v25 = -1476395002, v29 = 16
;; @001f                               v105 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v31 = load.i64 notrap aligned readonly can_move v105+32
;; @001f                               v32 = uextend.i64 v30
;; @001f                               v33 = iadd v31, v32
;; @001f                               jump block4(v30, v33)
;;
;;                                 block4(v42: i32, v43: i64):
;;                                     v104 = stack_addr.i64 ss0
;;                                     store notrap v42, v104
;; @001f                               v44 = iconst.i64 16
;; @001f                               v45 = iadd v43, v44  ; v44 = 16
;; @001f                               store.i32 user2 region1 v2, v45
;; @001f                               trapz v42, user16
;;                                     v154 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v155 = load.i64 notrap aligned readonly can_move v154+32
;; @001f                               v48 = uextend.i64 v42
;; @001f                               v50 = iadd v155, v48
;; @001f                               v52 = iadd v50, v44  ; v44 = 16
;; @001f                               v53 = load.i32 user2 readonly region1 v52
;; @001f                               v54 = uextend.i64 v53
;; @001f                               v60 = icmp.i64 ugt v5, v54
;; @001f                               trapnz v60, user17
;; @001f                               v74 = load.i64 notrap aligned v154+40
;; @001f                               v64 = iconst.i64 20
;; @001f                               v65 = iadd v50, v64  ; v64 = 20
;; @001f                               v76 = uadd_overflow_trap v65, v111, user2
;; @001f                               v75 = iadd v155, v74
;; @001f                               v77 = icmp ugt v76, v75
;; @001f                               trapnz v77, user2
;; @001f                               v46 = iconst.i64 0
;; @001f                               v79 = call fn1(v0, v46), stack_map=[i32 @ ss0+0]  ; v46 = 0
;; @001f                               v83 = icmp.i64 eq v5, v46  ; v46 = 0
;; @001f                               v80 = ireduce.i32 v79
;; @001f                               v6 = iconst.i64 4
;; @001f                               v81 = iadd v65, v111
;; @001f                               brif v83, block6, block5(v65)
;;
;;                                 block5(v84: i64):
;; @001f                               store.i32 notrap aligned little v80, v84
;;                                     v156 = iconst.i64 4
;;                                     v157 = iadd v84, v156  ; v156 = 4
;; @001f                               v87 = icmp eq v157, v81
;; @001f                               brif v87, block6, block5(v157)
;;
;;                                 block6:
;;                                     v88 = load.i32 notrap v104
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v88
;; }
