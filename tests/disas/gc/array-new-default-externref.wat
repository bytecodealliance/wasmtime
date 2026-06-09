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
;;                                     v90 = iconst.i64 2
;;                                     v91 = ishl v5, v90  ; v90 = 2
;; @001f                               v8 = iconst.i64 32
;; @001f                               v9 = ushr v91, v8  ; v8 = 32
;; @001f                               trapnz v9, user18
;; @001f                               v4 = iconst.i32 20
;;                                     v97 = iconst.i32 2
;;                                     v98 = ishl v2, v97  ; v97 = 2
;; @001f                               v11 = uadd_overflow_trap v4, v98, user18  ; v4 = 20
;; @001f                               v12 = load.i64 notrap aligned readonly can_move v0+32
;; @001f                               v13 = load.i32 notrap aligned v12
;; @001f                               v14 = load.i32 notrap aligned v12+4
;; @001f                               v20 = uextend.i64 v13
;; @001f                               v15 = uextend.i64 v11
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
;;                                     v106 = iconst.i32 15
;;                                     v107 = iadd.i32 v11, v106  ; v106 = 15
;;                                     v110 = iconst.i32 -16
;;                                     v111 = band v107, v110  ; v110 = -16
;;                                     v113 = iadd.i32 v13, v111
;; @001f                               store notrap aligned region0 v113, v12
;;                                     v129 = iconst.i32 -1476394994
;;                                     v130 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v131 = load.i64 notrap aligned readonly can_move v130+32
;; @001f                               v37 = iadd v131, v20
;; @001f                               store notrap aligned v129, v37  ; v129 = -1476394994
;;                                     v132 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v133 = load.i32 notrap aligned readonly can_move v132
;; @001f                               store notrap aligned v133, v37+4
;;                                     v134 = band.i64 v18, v17  ; v17 = -16
;; @001f                               istore32 notrap aligned v134, v37+8
;; @001f                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @001f                               v24 = iconst.i32 -1476394994
;; @001f                               v25 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v26 = load.i32 notrap aligned readonly can_move v25
;; @001f                               v27 = iconst.i32 16
;; @001f                               v28 = call fn0(v0, v24, v26, v11, v27)  ; v24 = -1476394994, v27 = 16
;; @001f                               v29 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v30 = load.i64 notrap aligned readonly can_move v29+32
;; @001f                               v31 = uextend.i64 v28
;; @001f                               v32 = iadd v30, v31
;; @001f                               jump block4(v28, v32)
;;
;;                                 block4(v41: i32, v42: i64):
;; @001f                               v43 = iconst.i64 16
;; @001f                               v44 = iadd v42, v43  ; v43 = 16
;; @001f                               store.i32 user2 region1 v2, v44
;; @001f                               trapz v41, user16
;;                                     v135 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v136 = load.i64 notrap aligned readonly can_move v135+32
;; @001f                               v47 = uextend.i64 v41
;; @001f                               v49 = iadd v136, v47
;; @001f                               v51 = iadd v49, v43  ; v43 = 16
;; @001f                               v52 = load.i32 user2 readonly region1 v51
;; @001f                               v53 = uextend.i64 v52
;; @001f                               v59 = icmp.i64 ugt v5, v53
;; @001f                               trapnz v59, user17
;; @001f                               v75 = load.i64 notrap aligned v135+40
;; @001f                               v64 = iconst.i64 20
;; @001f                               v65 = iadd v49, v64  ; v64 = 20
;; @001f                               v77 = uadd_overflow_trap v65, v91, user2
;; @001f                               v76 = iadd v136, v75
;; @001f                               v78 = icmp ugt v77, v76
;; @001f                               trapnz v78, user2
;;                                     v115 = iconst.i64 0
;; @001f                               v81 = icmp.i64 eq v5, v115  ; v115 = 0
;; @001f                               v45 = iconst.i32 0
;; @001f                               v6 = iconst.i64 4
;; @001f                               v79 = iadd v65, v91
;; @001f                               brif v81, block6, block5(v65)
;;
;;                                 block5(v82: i64):
;;                                     v137 = iconst.i32 0
;; @001f                               store user2 little region1 v137, v82  ; v137 = 0
;;                                     v138 = iconst.i64 4
;;                                     v139 = iadd v82, v138  ; v138 = 4
;; @001f                               v85 = icmp eq v139, v79
;; @001f                               brif v85, block6, block5(v139)
;;
;;                                 block6:
;; @0022                               jump block1(v41)
;;
;;                                 block1(v3: i32):
;; @0022                               return v3
;; }
