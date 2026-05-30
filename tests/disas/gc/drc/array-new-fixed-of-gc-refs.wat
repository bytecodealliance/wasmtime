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
;;     region0 = 2147483648 "GcHeap"
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v220 = stack_addr.i64 ss2
;;                                     store notrap v2, v220
;;                                     v219 = stack_addr.i64 ss1
;;                                     store notrap v3, v219
;;                                     v218 = stack_addr.i64 ss0
;;                                     store notrap v4, v218
;; @0025                               v15 = iconst.i32 -1476395008
;; @0025                               v17 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v18 = load.i32 notrap aligned readonly can_move v17
;;                                     v232 = iconst.i32 40
;; @0025                               v19 = iconst.i32 8
;; @0025                               v20 = call fn0(v0, v15, v18, v232, v19), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v15 = -1476395008, v232 = 40, v19 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v215 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v21 = load.i64 notrap aligned readonly can_move v215+32
;; @0025                               v22 = uextend.i64 v20
;; @0025                               v23 = iadd v21, v22
;; @0025                               v24 = iconst.i64 24
;; @0025                               v25 = iadd v23, v24  ; v24 = 24
;; @0025                               store user2 region0 v6, v25  ; v6 = 3
;; @0025                               trapz v20, user16
;; @0025                               v44 = uadd_overflow_trap v20, v232, user2  ; v232 = 40
;;                                     v172 = load.i32 notrap v220
;; @0025                               v51 = iconst.i32 1
;; @0025                               v52 = band v172, v51  ; v51 = 1
;; @0025                               v26 = iconst.i32 0
;; @0025                               v54 = icmp eq v172, v26  ; v26 = 0
;; @0025                               v55 = uextend.i32 v54
;; @0025                               v56 = bor v52, v55
;; @0025                               brif v56, block3, block2
;;
;;                                 block2:
;;                                     v170 = load.i32 notrap v220
;; @0025                               v57 = uextend.i64 v170
;; @0025                               v59 = iadd.i64 v21, v57
;; @0025                               v60 = iconst.i64 8
;; @0025                               v61 = iadd v59, v60  ; v60 = 8
;; @0025                               v62 = load.i64 user2 region0 v61
;; @0025                               v63 = iconst.i64 1
;; @0025                               v64 = iadd v62, v63  ; v63 = 1
;; @0025                               store user2 region0 v64, v61
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v168 = load.i32 notrap v220
;; @0025                               v45 = uextend.i64 v44
;; @0025                               v47 = iadd.i64 v21, v45
;;                                     v222 = iconst.i64 12
;; @0025                               v50 = isub v47, v222  ; v222 = 12
;; @0025                               store user2 little region0 v168, v50
;;                                     v325 = iadd.i64 v23, v24  ; v24 = 24
;; @0025                               v76 = load.i32 user2 readonly region0 v325
;;                                     v326 = iconst.i32 1
;;                                     v327 = icmp ugt v76, v326  ; v326 = 1
;; @0025                               trapz v327, user17
;; @0025                               v79 = uextend.i64 v76
;;                                     v223 = iconst.i64 2
;;                                     v267 = ishl v79, v223  ; v223 = 2
;;                                     v217 = iconst.i64 32
;; @0025                               v81 = ushr v267, v217  ; v217 = 32
;; @0025                               trapnz v81, user2
;;                                     v244 = iconst.i32 2
;;                                     v274 = ishl v76, v244  ; v244 = 2
;; @0025                               v7 = iconst.i32 28
;; @0025                               v84 = uadd_overflow_trap v274, v7, user2  ; v7 = 28
;; @0025                               v88 = uadd_overflow_trap.i32 v20, v84, user2
;;                                     v167 = load.i32 notrap v219
;;                                     v328 = band v167, v326  ; v326 = 1
;;                                     v329 = iconst.i32 0
;;                                     v330 = icmp eq v167, v329  ; v329 = 0
;; @0025                               v99 = uextend.i32 v330
;; @0025                               v100 = bor v328, v99
;; @0025                               brif v100, block5, block4
;;
;;                                 block4:
;;                                     v165 = load.i32 notrap v219
;; @0025                               v101 = uextend.i64 v165
;; @0025                               v103 = iadd.i64 v21, v101
;;                                     v331 = iconst.i64 8
;; @0025                               v105 = iadd v103, v331  ; v331 = 8
;; @0025                               v106 = load.i64 user2 region0 v105
;;                                     v332 = iconst.i64 1
;; @0025                               v108 = iadd v106, v332  ; v332 = 1
;; @0025                               store user2 region0 v108, v105
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v163 = load.i32 notrap v219
;; @0025                               v89 = uextend.i64 v88
;; @0025                               v91 = iadd.i64 v21, v89
;;                                     v287 = iconst.i32 32
;; @0025                               v92 = isub.i32 v84, v287  ; v287 = 32
;; @0025                               v93 = uextend.i64 v92
;; @0025                               v94 = isub v91, v93
;; @0025                               store user2 little region0 v163, v94
;;                                     v333 = iadd.i64 v23, v24  ; v24 = 24
;; @0025                               v120 = load.i32 user2 readonly region0 v333
;;                                     v334 = iconst.i32 2
;;                                     v335 = icmp ugt v120, v334  ; v334 = 2
;; @0025                               trapz v335, user17
;; @0025                               v123 = uextend.i64 v120
;;                                     v336 = iconst.i64 2
;;                                     v337 = ishl v123, v336  ; v336 = 2
;;                                     v338 = iconst.i64 32
;;                                     v339 = ushr v337, v338  ; v338 = 32
;; @0025                               trapnz v339, user2
;;                                     v340 = ishl v120, v334  ; v334 = 2
;;                                     v341 = iconst.i32 28
;; @0025                               v128 = uadd_overflow_trap v340, v341, user2  ; v341 = 28
;; @0025                               v132 = uadd_overflow_trap.i32 v20, v128, user2
;;                                     v162 = load.i32 notrap v218
;;                                     v342 = iconst.i32 1
;;                                     v343 = band v162, v342  ; v342 = 1
;;                                     v344 = iconst.i32 0
;;                                     v345 = icmp eq v162, v344  ; v344 = 0
;; @0025                               v143 = uextend.i32 v345
;; @0025                               v144 = bor v343, v143
;; @0025                               brif v144, block7, block6
;;
;;                                 block6:
;;                                     v160 = load.i32 notrap v218
;; @0025                               v145 = uextend.i64 v160
;; @0025                               v147 = iadd.i64 v21, v145
;;                                     v346 = iconst.i64 8
;; @0025                               v149 = iadd v147, v346  ; v346 = 8
;; @0025                               v150 = load.i64 user2 region0 v149
;;                                     v347 = iconst.i64 1
;; @0025                               v152 = iadd v150, v347  ; v347 = 1
;; @0025                               store user2 region0 v152, v149
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v158 = load.i32 notrap v218
;; @0025                               v133 = uextend.i64 v132
;; @0025                               v135 = iadd.i64 v21, v133
;;                                     v319 = iconst.i32 36
;; @0025                               v136 = isub.i32 v128, v319  ; v319 = 36
;; @0025                               v137 = uextend.i64 v136
;; @0025                               v138 = isub v135, v137
;; @0025                               store user2 little region0 v158, v138
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v20
;; }
