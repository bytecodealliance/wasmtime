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
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v232 = iconst.i32 40
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v14, v17, v232, v18), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v14 = -1476395008, v232 = 40, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v214 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v20 = load.i64 notrap aligned readonly can_move v214+32
;; @0025                               v21 = uextend.i64 v19
;; @0025                               v22 = iadd v20, v21
;; @0025                               v23 = iconst.i64 24
;; @0025                               v24 = iadd v22, v23  ; v23 = 24
;; @0025                               store user2 region0 v6, v24  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v43 = uadd_overflow_trap v19, v232, user2  ; v232 = 40
;;                                     v168 = load.i32 notrap v220
;;                                     v207 = iconst.i32 1
;; @0025                               v50 = band v168, v207  ; v207 = 1
;; @0025                               v25 = iconst.i32 0
;; @0025                               v52 = icmp eq v168, v25  ; v25 = 0
;; @0025                               v53 = uextend.i32 v52
;; @0025                               v54 = bor v50, v53
;; @0025                               brif v54, block3, block2
;;
;;                                 block2:
;;                                     v166 = load.i32 notrap v220
;; @0025                               v55 = uextend.i64 v166
;; @0025                               v57 = iadd.i64 v20, v55
;; @0025                               v58 = iconst.i64 8
;; @0025                               v59 = iadd v57, v58  ; v58 = 8
;; @0025                               v60 = load.i64 user2 region0 v59
;; @0025                               v61 = iconst.i64 1
;; @0025                               v62 = iadd v60, v61  ; v61 = 1
;; @0025                               store user2 region0 v62, v59
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v164 = load.i32 notrap v220
;; @0025                               v44 = uextend.i64 v43
;; @0025                               v46 = iadd.i64 v20, v44
;;                                     v222 = iconst.i64 12
;; @0025                               v49 = isub v46, v222  ; v222 = 12
;; @0025                               store user2 little region0 v164, v49
;;                                     v325 = iadd.i64 v22, v23  ; v23 = 24
;; @0025                               v74 = load.i32 user2 readonly region0 v325
;;                                     v326 = iconst.i32 1
;;                                     v327 = icmp ugt v74, v326  ; v326 = 1
;; @0025                               trapz v327, user17
;; @0025                               v77 = uextend.i64 v74
;;                                     v223 = iconst.i64 2
;;                                     v267 = ishl v77, v223  ; v223 = 2
;;                                     v216 = iconst.i64 32
;; @0025                               v79 = ushr v267, v216  ; v216 = 32
;; @0025                               trapnz v79, user2
;;                                     v244 = iconst.i32 2
;;                                     v274 = ishl v74, v244  ; v244 = 2
;; @0025                               v7 = iconst.i32 28
;; @0025                               v82 = uadd_overflow_trap v274, v7, user2  ; v7 = 28
;; @0025                               v86 = uadd_overflow_trap.i32 v19, v82, user2
;;                                     v163 = load.i32 notrap v219
;;                                     v328 = band v163, v326  ; v326 = 1
;;                                     v329 = iconst.i32 0
;;                                     v330 = icmp eq v163, v329  ; v329 = 0
;; @0025                               v96 = uextend.i32 v330
;; @0025                               v97 = bor v328, v96
;; @0025                               brif v97, block5, block4
;;
;;                                 block4:
;;                                     v161 = load.i32 notrap v219
;; @0025                               v98 = uextend.i64 v161
;; @0025                               v100 = iadd.i64 v20, v98
;;                                     v331 = iconst.i64 8
;; @0025                               v102 = iadd v100, v331  ; v331 = 8
;; @0025                               v103 = load.i64 user2 region0 v102
;;                                     v332 = iconst.i64 1
;; @0025                               v105 = iadd v103, v332  ; v332 = 1
;; @0025                               store user2 region0 v105, v102
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v159 = load.i32 notrap v219
;; @0025                               v87 = uextend.i64 v86
;; @0025                               v89 = iadd.i64 v20, v87
;;                                     v287 = iconst.i32 32
;; @0025                               v90 = isub.i32 v82, v287  ; v287 = 32
;; @0025                               v91 = uextend.i64 v90
;; @0025                               v92 = isub v89, v91
;; @0025                               store user2 little region0 v159, v92
;;                                     v333 = iadd.i64 v22, v23  ; v23 = 24
;; @0025                               v117 = load.i32 user2 readonly region0 v333
;;                                     v334 = iconst.i32 2
;;                                     v335 = icmp ugt v117, v334  ; v334 = 2
;; @0025                               trapz v335, user17
;; @0025                               v120 = uextend.i64 v117
;;                                     v336 = iconst.i64 2
;;                                     v337 = ishl v120, v336  ; v336 = 2
;;                                     v338 = iconst.i64 32
;;                                     v339 = ushr v337, v338  ; v338 = 32
;; @0025                               trapnz v339, user2
;;                                     v340 = ishl v117, v334  ; v334 = 2
;;                                     v341 = iconst.i32 28
;; @0025                               v125 = uadd_overflow_trap v340, v341, user2  ; v341 = 28
;; @0025                               v129 = uadd_overflow_trap.i32 v19, v125, user2
;;                                     v158 = load.i32 notrap v218
;;                                     v342 = iconst.i32 1
;;                                     v343 = band v158, v342  ; v342 = 1
;;                                     v344 = iconst.i32 0
;;                                     v345 = icmp eq v158, v344  ; v344 = 0
;; @0025                               v139 = uextend.i32 v345
;; @0025                               v140 = bor v343, v139
;; @0025                               brif v140, block7, block6
;;
;;                                 block6:
;;                                     v156 = load.i32 notrap v218
;; @0025                               v141 = uextend.i64 v156
;; @0025                               v143 = iadd.i64 v20, v141
;;                                     v346 = iconst.i64 8
;; @0025                               v145 = iadd v143, v346  ; v346 = 8
;; @0025                               v146 = load.i64 user2 region0 v145
;;                                     v347 = iconst.i64 1
;; @0025                               v148 = iadd v146, v347  ; v347 = 1
;; @0025                               store user2 region0 v148, v145
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v154 = load.i32 notrap v218
;; @0025                               v130 = uextend.i64 v129
;; @0025                               v132 = iadd.i64 v20, v130
;;                                     v319 = iconst.i32 36
;; @0025                               v133 = isub.i32 v125, v319  ; v319 = 36
;; @0025                               v134 = uextend.i64 v133
;; @0025                               v135 = isub v132, v134
;; @0025                               store user2 little region0 v154, v135
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
