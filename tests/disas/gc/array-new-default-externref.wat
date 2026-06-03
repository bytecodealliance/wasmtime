;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut externref)))

  (func $fill (param $len i32) (result (ref $a))
    (array.new_default $a (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               v5 = uextend.i64 v2
;;                                     v98 = iconst.i64 2
;;                                     v99 = ishl v5, v98  ; v98 = 2
;;                                     v97 = iconst.i64 32
;; @001f                               v8 = ushr v99, v97  ; v97 = 32
;; @001f                               trapnz v8, user18
;; @001f                               v4 = iconst.i32 20
;;                                     v105 = iconst.i32 2
;;                                     v106 = ishl v2, v105  ; v105 = 2
;; @001f                               v10 = uadd_overflow_trap v4, v106, user18  ; v4 = 20
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
;;                                     v114 = iconst.i32 15
;;                                     v115 = iadd.i32 v10, v114  ; v114 = 15
;;                                     v118 = iconst.i32 -16
;;                                     v119 = band v115, v118  ; v118 = -16
;;                                     v121 = iadd.i32 v13, v119
;; @001f                               store notrap aligned region0 v121, v12
;;                                     v137 = iconst.i32 -1476394994
;;                                     v138 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v139 = load.i64 notrap aligned readonly can_move v138+32
;; @001f                               v37 = iadd v139, v20
;; @001f                               store notrap aligned v137, v37  ; v137 = -1476394994
;;                                     v140 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v141 = load.i32 notrap aligned readonly can_move v140
;; @001f                               store notrap aligned v141, v37+4
;;                                     v142 = band.i64 v18, v17  ; v17 = -16
;; @001f                               istore32 notrap aligned v142, v37+8
;; @001f                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @001f                               v25 = iconst.i32 -1476394994
;; @001f                               v27 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v28 = load.i32 notrap aligned readonly can_move v27
;; @001f                               v29 = iconst.i32 16
;; @001f                               v30 = call fn0(v0, v25, v28, v10, v29)  ; v25 = -1476394994, v29 = 16
;; @001f                               v93 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v31 = load.i64 notrap aligned readonly can_move v93+32
;; @001f                               v32 = uextend.i64 v30
;; @001f                               v33 = iadd v31, v32
;; @001f                               jump block4(v30, v33)
;;
;;                                 block4(v42: i32, v43: i64):
;; @001f                               v44 = iconst.i64 16
;; @001f                               v45 = iadd v43, v44  ; v44 = 16
;; @001f                               store.i32 user2 region1 v2, v45
;; @001f                               trapz v42, user16
;;                                     v143 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v144 = load.i64 notrap aligned readonly can_move v143+32
;; @001f                               v48 = uextend.i64 v42
;; @001f                               v50 = iadd v144, v48
;; @001f                               v52 = iadd v50, v44  ; v44 = 16
;; @001f                               v53 = load.i32 user2 readonly region1 v52
;; @001f                               v54 = uextend.i64 v53
;; @001f                               v60 = icmp.i64 ugt v5, v54
;; @001f                               trapnz v60, user17
;; @001f                               v74 = load.i64 notrap aligned v143+40
;; @001f                               v64 = iconst.i64 20
;; @001f                               v65 = iadd v50, v64  ; v64 = 20
;; @001f                               v76 = uadd_overflow_trap v65, v99, user2
;; @001f                               v75 = iadd v144, v74
;; @001f                               v77 = icmp ugt v76, v75
;; @001f                               trapnz v77, user2
;;                                     v123 = iconst.i64 0
;; @001f                               v80 = icmp.i64 eq v5, v123  ; v123 = 0
;; @001f                               v46 = iconst.i32 0
;; @001f                               v6 = iconst.i64 4
;; @001f                               v78 = iadd v65, v99
;; @001f                               brif v80, block6, block5(v65)
;;
;;                                 block5(v81: i64):
;;                                     v145 = iconst.i32 0
;; @001f                               store user2 little region1 v145, v81  ; v145 = 0
;;                                     v146 = iconst.i64 4
;;                                     v147 = iadd v81, v146  ; v146 = 4
;; @001f                               v84 = icmp eq v147, v78
;; @001f                               brif v84, block6, block5(v147)
;;
;;                                 block6:
;; @0022                               jump block1(v42)
;;
;;                                 block1(v3: i32):
;; @0022                               return v3
;; }
