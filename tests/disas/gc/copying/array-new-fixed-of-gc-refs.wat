;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut anyref)))

  (func (param anyref anyref anyref) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     ss1 = explicit_slot 4, align = 4
;;     ss2 = explicit_slot 4, align = 4
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v132 = stack_addr.i64 ss2
;;                                     store notrap v2, v132
;;                                     v133 = stack_addr.i64 ss1
;;                                     store notrap v3, v133
;;                                     v134 = stack_addr.i64 ss0
;;                                     store notrap v4, v134
;; @0025                               v15 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v16 = load.i32 notrap aligned v15
;; @0025                               v17 = load.i32 notrap aligned v15+4
;; @0025                               v23 = uextend.i64 v16
;; @0025                               v11 = iconst.i64 32
;; @0025                               v24 = iadd v23, v11  ; v11 = 32
;; @0025                               v25 = uextend.i64 v17
;; @0025                               v26 = icmp ule v24, v25
;; @0025                               brif v26, block2, block3
;;
;;                                 block2:
;;                                     v266 = iconst.i32 32
;;                                     v172 = iadd.i32 v16, v266  ; v266 = 32
;; @0025                               store notrap aligned region2 v172, v15
;;                                     v267 = iconst.i32 -1476394994
;;                                     v268 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v269 = load.i64 notrap aligned readonly can_move v268+32
;; @0025                               v40 = iadd v269, v23
;; @0025                               store notrap aligned v267, v40  ; v267 = -1476394994
;;                                     v270 = load.i64 notrap aligned readonly can_move region1 v0+40
;;                                     v271 = load.i32 notrap aligned readonly can_move v270
;; @0025                               store notrap aligned v271, v40+4
;;                                     v272 = iconst.i64 32
;; @0025                               istore32 notrap aligned v272, v40+8  ; v272 = 32
;; @0025                               jump block4(v16, v40)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476394994
;; @0025                               v28 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @0025                               v29 = load.i32 notrap aligned readonly can_move v28
;;                                     v158 = iconst.i32 32
;; @0025                               v30 = iconst.i32 16
;; @0025                               v31 = call fn0(v0, v27, v29, v158, v30), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v27 = -1476394994, v158 = 32, v30 = 16
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
;;                                     v273 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v274 = load.i64 notrap aligned readonly can_move v273+32
;; @0025                               v49 = uextend.i64 v44
;; @0025                               v51 = iadd v274, v49
;; @0025                               v53 = iadd v51, v46  ; v46 = 16
;; @0025                               v54 = load.i32 user2 readonly region3 v53
;; @0025                               trapz v54, user17
;; @0025                               v57 = uextend.i64 v54
;;                                     v149 = iconst.i64 2
;;                                     v178 = ishl v57, v149  ; v149 = 2
;;                                     v275 = iconst.i64 32
;;                                     v276 = ushr v178, v275  ; v275 = 32
;; @0025                               trapnz v276, user2
;;                                     v187 = iconst.i32 2
;;                                     v188 = ishl v54, v187  ; v187 = 2
;; @0025                               v7 = iconst.i32 20
;; @0025                               v63 = uadd_overflow_trap v188, v7, user2  ; v7 = 20
;; @0025                               v67 = uadd_overflow_trap v44, v63, user2
;;                                     v131 = load.i32 notrap v132
;; @0025                               v68 = uextend.i64 v67
;; @0025                               v70 = iadd v274, v68
;; @0025                               v71 = isub v63, v7  ; v7 = 20
;; @0025                               v72 = uextend.i64 v71
;; @0025                               v73 = isub v70, v72
;; @0025                               store user2 little region3 v131, v73
;; @0025                               v80 = load.i32 user2 readonly region3 v53
;; @0025                               v74 = iconst.i32 1
;;                                     v205 = icmp ugt v80, v74  ; v74 = 1
;; @0025                               trapz v205, user17
;; @0025                               v83 = uextend.i64 v80
;;                                     v207 = ishl v83, v149  ; v149 = 2
;;                                     v277 = ushr v207, v275  ; v275 = 32
;; @0025                               trapnz v277, user2
;;                                     v214 = ishl v80, v187  ; v187 = 2
;; @0025                               v89 = uadd_overflow_trap v214, v7, user2  ; v7 = 20
;; @0025                               v93 = uadd_overflow_trap v44, v89, user2
;;                                     v129 = load.i32 notrap v133
;; @0025                               v94 = uextend.i64 v93
;; @0025                               v96 = iadd v274, v94
;;                                     v227 = iconst.i32 24
;; @0025                               v97 = isub v89, v227  ; v227 = 24
;; @0025                               v98 = uextend.i64 v97
;; @0025                               v99 = isub v96, v98
;; @0025                               store user2 little region3 v129, v99
;; @0025                               v106 = load.i32 user2 readonly region3 v53
;;                                     v233 = icmp ugt v106, v187  ; v187 = 2
;; @0025                               trapz v233, user17
;; @0025                               v109 = uextend.i64 v106
;;                                     v235 = ishl v109, v149  ; v149 = 2
;;                                     v278 = ushr v235, v275  ; v275 = 32
;; @0025                               trapnz v278, user2
;;                                     v242 = ishl v106, v187  ; v187 = 2
;; @0025                               v115 = uadd_overflow_trap v242, v7, user2  ; v7 = 20
;; @0025                               v119 = uadd_overflow_trap v44, v115, user2
;;                                     v127 = load.i32 notrap v134
;; @0025                               v120 = uextend.i64 v119
;; @0025                               v122 = iadd v274, v120
;;                                     v260 = iconst.i32 28
;; @0025                               v123 = isub v115, v260  ; v260 = 28
;; @0025                               v124 = uextend.i64 v123
;; @0025                               v125 = isub v122, v124
;; @0025                               store user2 little region3 v127, v125
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
