;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     region2 = 2147483648 "GcHeap"
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
;;                                     v191 = stack_addr.i64 ss2
;;                                     store notrap v2, v191
;;                                     v192 = stack_addr.i64 ss1
;;                                     store notrap v3, v192
;;                                     v193 = stack_addr.i64 ss0
;;                                     store notrap v4, v193
;; @0025                               v15 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v229 = iconst.i32 40
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v15, v17, v229, v18), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v15 = -1476395008, v229 = 40, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v20 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v21 = load.i64 notrap aligned readonly can_move v20+32
;; @0025                               v22 = uextend.i64 v19
;; @0025                               v23 = iadd v21, v22
;; @0025                               v24 = iconst.i64 24
;; @0025                               v25 = iadd v23, v24  ; v24 = 24
;; @0025                               store user2 region2 v6, v25  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v45 = uadd_overflow_trap v19, v229, user2  ; v229 = 40
;;                                     v190 = load.i32 notrap v191
;; @0025                               v52 = iconst.i32 1
;; @0025                               v53 = band v190, v52  ; v52 = 1
;; @0025                               v26 = iconst.i32 0
;; @0025                               v55 = icmp eq v190, v26  ; v26 = 0
;; @0025                               v56 = uextend.i32 v55
;; @0025                               v57 = bor v53, v56
;; @0025                               brif v57, block3, block2
;;
;;                                 block2:
;;                                     v186 = load.i32 notrap v191
;; @0025                               v58 = uextend.i64 v186
;; @0025                               v60 = iadd.i64 v21, v58
;; @0025                               v61 = iconst.i64 8
;; @0025                               v62 = iadd v60, v61  ; v61 = 8
;; @0025                               v63 = load.i64 user2 region2 v62
;; @0025                               v64 = iconst.i64 1
;; @0025                               v65 = iadd v63, v64  ; v64 = 1
;; @0025                               store user2 region2 v65, v62
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v182 = load.i32 notrap v191
;; @0025                               v46 = uextend.i64 v45
;; @0025                               v48 = iadd.i64 v21, v46
;;                                     v219 = iconst.i64 12
;; @0025                               v51 = isub v48, v219  ; v219 = 12
;; @0025                               store user2 little region2 v182, v51
;;                                     v322 = iadd.i64 v23, v24  ; v24 = 24
;; @0025                               v77 = load.i32 user2 readonly region2 v322
;;                                     v323 = iconst.i32 1
;;                                     v324 = icmp ugt v77, v323  ; v323 = 1
;; @0025                               trapz v324, user17
;; @0025                               v80 = uextend.i64 v77
;;                                     v220 = iconst.i64 2
;;                                     v264 = ishl v80, v220  ; v220 = 2
;; @0025                               v11 = iconst.i64 32
;; @0025                               v83 = ushr v264, v11  ; v11 = 32
;; @0025                               trapnz v83, user2
;;                                     v241 = iconst.i32 2
;;                                     v271 = ishl v77, v241  ; v241 = 2
;; @0025                               v7 = iconst.i32 28
;; @0025                               v86 = uadd_overflow_trap v271, v7, user2  ; v7 = 28
;; @0025                               v90 = uadd_overflow_trap.i32 v19, v86, user2
;;                                     v180 = load.i32 notrap v192
;;                                     v325 = band v180, v323  ; v323 = 1
;;                                     v326 = iconst.i32 0
;;                                     v327 = icmp eq v180, v326  ; v326 = 0
;; @0025                               v101 = uextend.i32 v327
;; @0025                               v102 = bor v325, v101
;; @0025                               brif v102, block5, block4
;;
;;                                 block4:
;;                                     v176 = load.i32 notrap v192
;; @0025                               v103 = uextend.i64 v176
;; @0025                               v105 = iadd.i64 v21, v103
;;                                     v328 = iconst.i64 8
;; @0025                               v107 = iadd v105, v328  ; v328 = 8
;; @0025                               v108 = load.i64 user2 region2 v107
;;                                     v329 = iconst.i64 1
;; @0025                               v110 = iadd v108, v329  ; v329 = 1
;; @0025                               store user2 region2 v110, v107
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v172 = load.i32 notrap v192
;; @0025                               v91 = uextend.i64 v90
;; @0025                               v93 = iadd.i64 v21, v91
;;                                     v284 = iconst.i32 32
;; @0025                               v94 = isub.i32 v86, v284  ; v284 = 32
;; @0025                               v95 = uextend.i64 v94
;; @0025                               v96 = isub v93, v95
;; @0025                               store user2 little region2 v172, v96
;;                                     v330 = iadd.i64 v23, v24  ; v24 = 24
;; @0025                               v122 = load.i32 user2 readonly region2 v330
;;                                     v331 = iconst.i32 2
;;                                     v332 = icmp ugt v122, v331  ; v331 = 2
;; @0025                               trapz v332, user17
;; @0025                               v125 = uextend.i64 v122
;;                                     v333 = iconst.i64 2
;;                                     v334 = ishl v125, v333  ; v333 = 2
;;                                     v335 = iconst.i64 32
;;                                     v336 = ushr v334, v335  ; v335 = 32
;; @0025                               trapnz v336, user2
;;                                     v337 = ishl v122, v331  ; v331 = 2
;;                                     v338 = iconst.i32 28
;; @0025                               v131 = uadd_overflow_trap v337, v338, user2  ; v338 = 28
;; @0025                               v135 = uadd_overflow_trap.i32 v19, v131, user2
;;                                     v170 = load.i32 notrap v193
;;                                     v339 = iconst.i32 1
;;                                     v340 = band v170, v339  ; v339 = 1
;;                                     v341 = iconst.i32 0
;;                                     v342 = icmp eq v170, v341  ; v341 = 0
;; @0025                               v146 = uextend.i32 v342
;; @0025                               v147 = bor v340, v146
;; @0025                               brif v147, block7, block6
;;
;;                                 block6:
;;                                     v166 = load.i32 notrap v193
;; @0025                               v148 = uextend.i64 v166
;; @0025                               v150 = iadd.i64 v21, v148
;;                                     v343 = iconst.i64 8
;; @0025                               v152 = iadd v150, v343  ; v343 = 8
;; @0025                               v153 = load.i64 user2 region2 v152
;;                                     v344 = iconst.i64 1
;; @0025                               v155 = iadd v153, v344  ; v344 = 1
;; @0025                               store user2 region2 v155, v152
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v162 = load.i32 notrap v193
;; @0025                               v136 = uextend.i64 v135
;; @0025                               v138 = iadd.i64 v21, v136
;;                                     v316 = iconst.i32 36
;; @0025                               v139 = isub.i32 v131, v316  ; v316 = 36
;; @0025                               v140 = uextend.i64 v139
;; @0025                               v141 = isub v138, v140
;; @0025                               store user2 little region2 v162, v141
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
