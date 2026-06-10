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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 40 "VMContext+0x28"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               v5 = uextend.i64 v2
;;                                     v89 = iconst.i64 2
;;                                     v90 = ishl v5, v89  ; v89 = 2
;; @001f                               v8 = iconst.i64 32
;; @001f                               v9 = ushr v90, v8  ; v8 = 32
;; @001f                               trapnz v9, user18
;; @001f                               v4 = iconst.i32 20
;;                                     v96 = iconst.i32 2
;;                                     v97 = ishl v2, v96  ; v96 = 2
;; @001f                               v11 = uadd_overflow_trap v4, v97, user18  ; v4 = 20
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
;;                                     v105 = iconst.i32 15
;;                                     v106 = iadd.i32 v11, v105  ; v105 = 15
;;                                     v109 = iconst.i32 -16
;;                                     v110 = band v106, v109  ; v109 = -16
;;                                     v112 = iadd.i32 v13, v110
;; @001f                               store notrap aligned region2 v112, v12
;;                                     v128 = iconst.i32 -1476394994
;;                                     v129 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v130 = load.i64 notrap aligned readonly can_move v129+32
;; @001f                               v37 = iadd v130, v20
;; @001f                               store notrap aligned v128, v37  ; v128 = -1476394994
;;                                     v131 = load.i64 notrap aligned readonly can_move region1 v0+40
;;                                     v132 = load.i32 notrap aligned readonly can_move v131
;; @001f                               store notrap aligned v132, v37+4
;;                                     v133 = band.i64 v18, v17  ; v17 = -16
;; @001f                               istore32 notrap aligned v133, v37+8
;; @001f                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @001f                               v24 = iconst.i32 -1476394994
;; @001f                               v25 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @001f                               v26 = load.i32 notrap aligned readonly can_move v25
;; @001f                               v27 = iconst.i32 16
;; @001f                               v28 = call fn0(v0, v24, v26, v11, v27)  ; v24 = -1476394994, v27 = 16
;; @001f                               v29 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001f                               v30 = load.i64 notrap aligned readonly can_move v29+32
;; @001f                               v31 = uextend.i64 v28
;; @001f                               v32 = iadd v30, v31
;; @001f                               jump block4(v28, v32)
;;
;;                                 block4(v41: i32, v42: i64):
;; @001f                               v43 = iconst.i64 16
;; @001f                               v44 = iadd v42, v43  ; v43 = 16
;; @001f                               store.i32 user2 region3 v2, v44
;; @001f                               trapz v41, user16
;;                                     v134 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v135 = load.i64 notrap aligned readonly can_move v134+32
;; @001f                               v47 = uextend.i64 v41
;; @001f                               v49 = iadd v135, v47
;; @001f                               v51 = iadd v49, v43  ; v43 = 16
;; @001f                               v52 = load.i32 user2 readonly region3 v51
;; @001f                               v53 = uextend.i64 v52
;; @001f                               v59 = icmp.i64 ugt v5, v53
;; @001f                               trapnz v59, user17
;; @001f                               v76 = load.i64 notrap aligned v134+40
;; @001f                               v64 = iconst.i64 20
;; @001f                               v65 = iadd v49, v64  ; v64 = 20
;; @001f                               v78 = uadd_overflow_trap v65, v90, user2
;; @001f                               v77 = iadd v135, v76
;; @001f                               v79 = icmp ugt v78, v77
;; @001f                               trapnz v79, user2
;;                                     v114 = iconst.i64 0
;; @001f                               v82 = icmp.i64 eq v5, v114  ; v114 = 0
;; @001f                               v45 = iconst.i32 0
;; @001f                               v6 = iconst.i64 4
;; @001f                               v80 = iadd v65, v90
;; @001f                               brif v82, block6, block5(v65)
;;
;;                                 block5(v83: i64):
;;                                     v136 = iconst.i32 0
;; @001f                               store user2 little region3 v136, v83  ; v136 = 0
;;                                     v137 = iconst.i64 4
;;                                     v138 = iadd v83, v137  ; v137 = 4
;; @001f                               v86 = icmp eq v138, v80
;; @001f                               brif v86, block6, block5(v138)
;;
;;                                 block6:
;; @0022                               jump block1(v41)
;;
;;                                 block1(v3: i32):
;; @0022                               return v3
;; }
