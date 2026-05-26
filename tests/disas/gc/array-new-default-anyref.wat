;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut anyref)))

  (func $fill (param $len i32) (result (ref $a))
    (array.new_default $a (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 2 "vmctx"
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
;;                                     v96 = iconst.i64 32
;; @001f                               v7 = ushr v99, v96  ; v96 = 32
;; @001f                               trapnz v7, user18
;; @001f                               v4 = iconst.i32 20
;;                                     v105 = iconst.i32 2
;;                                     v106 = ishl v2, v105  ; v105 = 2
;; @001f                               v9 = uadd_overflow_trap v4, v106, user18  ; v4 = 20
;; @001f                               v11 = load.i64 notrap aligned readonly can_move v0+32
;; @001f                               v12 = load.i32 notrap aligned v11
;; @001f                               v13 = load.i32 notrap aligned v11+4
;; @001f                               v19 = uextend.i64 v12
;; @001f                               v14 = uextend.i64 v9
;; @001f                               v15 = iconst.i64 15
;; @001f                               v17 = iadd v14, v15  ; v15 = 15
;; @001f                               v16 = iconst.i64 -16
;; @001f                               v18 = band v17, v16  ; v16 = -16
;; @001f                               v20 = iadd v19, v18
;; @001f                               v21 = uextend.i64 v13
;; @001f                               v22 = icmp ule v20, v21
;; @001f                               brif v22, block2, block3
;;
;;                                 block2:
;;                                     v114 = iconst.i32 15
;;                                     v115 = iadd.i32 v9, v114  ; v114 = 15
;;                                     v118 = iconst.i32 -16
;;                                     v119 = band v115, v118  ; v118 = -16
;;                                     v121 = iadd.i32 v12, v119
;; @001f                               store notrap aligned region0 v121, v11
;;                                     v137 = iconst.i32 -1476394994
;;                                     v138 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v139 = load.i64 notrap aligned readonly can_move v138+32
;; @001f                               v36 = iadd v139, v19
;; @001f                               store notrap aligned v137, v36  ; v137 = -1476394994
;;                                     v140 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v141 = load.i32 notrap aligned readonly can_move v140
;; @001f                               store notrap aligned v141, v36+4
;;                                     v142 = band.i64 v17, v16  ; v16 = -16
;; @001f                               istore32 notrap aligned v142, v36+8
;; @001f                               jump block4(v12, v36)
;;
;;                                 block3 cold:
;; @001f                               v24 = iconst.i32 -1476394994
;; @001f                               v26 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v27 = load.i32 notrap aligned readonly can_move v26
;; @001f                               v28 = iconst.i32 16
;; @001f                               v29 = call fn0(v0, v24, v27, v9, v28)  ; v24 = -1476394994, v28 = 16
;; @001f                               v92 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v30 = load.i64 notrap aligned readonly can_move v92+32
;; @001f                               v31 = uextend.i64 v29
;; @001f                               v32 = iadd v30, v31
;; @001f                               jump block4(v29, v32)
;;
;;                                 block4(v41: i32, v42: i64):
;;                                     v91 = iconst.i64 16
;; @001f                               v43 = iadd v42, v91  ; v91 = 16
;; @001f                               store.i32 user2 v2, v43
;; @001f                               trapz v41, user16
;;                                     v143 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v144 = load.i64 notrap aligned readonly can_move v143+32
;; @001f                               v46 = uextend.i64 v41
;; @001f                               v48 = iadd v144, v46
;; @001f                               v50 = iadd v48, v91  ; v91 = 16
;; @001f                               v51 = load.i32 user2 readonly v50
;; @001f                               v52 = uextend.i64 v51
;; @001f                               v57 = icmp.i64 ugt v5, v52
;; @001f                               trapnz v57, user17
;; @001f                               v68 = load.i64 notrap aligned v143+40
;;                                     v85 = iconst.i64 20
;; @001f                               v61 = iadd v48, v85  ; v85 = 20
;; @001f                               v70 = uadd_overflow_trap v61, v99, user2
;; @001f                               v69 = iadd v144, v68
;; @001f                               v71 = icmp ugt v70, v69
;; @001f                               trapnz v71, user2
;;                                     v123 = iconst.i64 0
;; @001f                               v73 = icmp.i64 eq v5, v123  ; v123 = 0
;; @001f                               v44 = iconst.i32 0
;;                                     v97 = iconst.i64 4
;; @001f                               v72 = iadd v61, v99
;; @001f                               brif v73, block6, block5(v61)
;;
;;                                 block5(v74: i64):
;;                                     v145 = iconst.i32 0
;; @001f                               store user2 little v145, v74  ; v145 = 0
;;                                     v146 = iconst.i64 4
;;                                     v147 = iadd v74, v146  ; v146 = 4
;; @001f                               v76 = icmp eq v147, v72
;; @001f                               brif v76, block6, block5(v147)
;;
;;                                 block6:
;; @0022                               jump block1(v41)
;;
;;                                 block1(v3: i32):
;; @0022                               return v3
;; }
