;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param i64 i64 i64) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i64, i64) -> i32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v15 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v16 = load.i32 notrap aligned v15
;; @0025                               v17 = load.i32 notrap aligned v15+4
;; @0025                               v23 = uextend.i64 v16
;;                                     v148 = iconst.i64 48
;; @0025                               v24 = iadd v23, v148  ; v148 = 48
;; @0025                               v25 = uextend.i64 v17
;; @0025                               v26 = icmp ule v24, v25
;; @0025                               brif v26, block2, block3
;;
;;                                 block2:
;;                                     v254 = iconst.i32 48
;;                                     v162 = iadd.i32 v16, v254  ; v254 = 48
;; @0025                               store notrap aligned region2 v162, v15
;;                                     v255 = iconst.i32 -1476395002
;;                                     v256 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v257 = load.i64 notrap aligned readonly can_move v256+32
;; @0025                               v40 = iadd v257, v23
;; @0025                               store notrap aligned v255, v40  ; v255 = -1476395002
;;                                     v258 = load.i64 notrap aligned readonly can_move region1 v0+40
;;                                     v259 = load.i32 notrap aligned readonly can_move v258
;; @0025                               store notrap aligned v259, v40+4
;;                                     v260 = iconst.i64 48
;; @0025                               istore32 notrap aligned v260, v40+8  ; v260 = 48
;; @0025                               jump block4(v16, v40)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476395002
;; @0025                               v28 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @0025                               v29 = load.i32 notrap aligned readonly can_move v28
;;                                     v147 = iconst.i32 48
;; @0025                               v30 = iconst.i32 16
;; @0025                               v31 = call fn0(v0, v27, v29, v147, v30)  ; v27 = -1476395002, v147 = 48, v30 = 16
;; @0025                               v32 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v32+32
;; @0025                               v34 = uextend.i64 v31
;; @0025                               v35 = iadd v33, v34
;; @0025                               jump block4(v31, v35)
;;
;;                                 block4(v44: i32, v45: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v46 = iconst.i64 16
;; @0025                               v47 = iadd v45, v46  ; v46 = 16
;; @0025                               store user2 region3 v6, v47  ; v6 = 3
;; @0025                               trapz v44, user16
;;                                     v261 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v262 = load.i64 notrap aligned readonly can_move v261+32
;; @0025                               v49 = uextend.i64 v44
;; @0025                               v51 = iadd v262, v49
;; @0025                               v53 = iadd v51, v46  ; v46 = 16
;; @0025                               v54 = load.i32 user2 readonly region3 v53
;; @0025                               trapz v54, user17
;; @0025                               v57 = uextend.i64 v54
;;                                     v138 = iconst.i64 3
;;                                     v168 = ishl v57, v138  ; v138 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v60 = ushr v168, v11  ; v11 = 32
;; @0025                               trapnz v60, user2
;;                                     v177 = ishl v54, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 24
;; @0025                               v63 = uadd_overflow_trap v177, v7, user2  ; v7 = 24
;; @0025                               v67 = uadd_overflow_trap v44, v63, user2
;; @0025                               v68 = uextend.i64 v67
;; @0025                               v70 = iadd v262, v68
;; @0025                               v71 = isub v63, v7  ; v7 = 24
;; @0025                               v72 = uextend.i64 v71
;; @0025                               v73 = isub v70, v72
;; @0025                               store.i64 user2 little region3 v2, v73
;; @0025                               v80 = load.i32 user2 readonly region3 v53
;; @0025                               v74 = iconst.i32 1
;;                                     v194 = icmp ugt v80, v74  ; v74 = 1
;; @0025                               trapz v194, user17
;; @0025                               v83 = uextend.i64 v80
;;                                     v196 = ishl v83, v138  ; v138 = 3
;; @0025                               v86 = ushr v196, v11  ; v11 = 32
;; @0025                               trapnz v86, user2
;;                                     v203 = ishl v80, v6  ; v6 = 3
;; @0025                               v89 = uadd_overflow_trap v203, v7, user2  ; v7 = 24
;; @0025                               v93 = uadd_overflow_trap v44, v89, user2
;; @0025                               v94 = uextend.i64 v93
;; @0025                               v96 = iadd v262, v94
;;                                     v216 = iconst.i32 32
;; @0025                               v97 = isub v89, v216  ; v216 = 32
;; @0025                               v98 = uextend.i64 v97
;; @0025                               v99 = isub v96, v98
;; @0025                               store.i64 user2 little region3 v3, v99
;; @0025                               v106 = load.i32 user2 readonly region3 v53
;; @0025                               v100 = iconst.i32 2
;;                                     v222 = icmp ugt v106, v100  ; v100 = 2
;; @0025                               trapz v222, user17
;; @0025                               v109 = uextend.i64 v106
;;                                     v224 = ishl v109, v138  ; v138 = 3
;; @0025                               v112 = ushr v224, v11  ; v11 = 32
;; @0025                               trapnz v112, user2
;;                                     v231 = ishl v106, v6  ; v6 = 3
;; @0025                               v115 = uadd_overflow_trap v231, v7, user2  ; v7 = 24
;; @0025                               v119 = uadd_overflow_trap v44, v115, user2
;; @0025                               v120 = uextend.i64 v119
;; @0025                               v122 = iadd v262, v120
;;                                     v248 = iconst.i32 40
;; @0025                               v123 = isub v115, v248  ; v248 = 40
;; @0025                               v124 = uextend.i64 v123
;; @0025                               v125 = isub v122, v124
;; @0025                               store.i64 user2 little region3 v4, v125
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
