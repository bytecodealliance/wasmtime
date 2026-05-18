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
;;                                     v213 = iconst.i64 24
;; @0025                               v23 = iadd v22, v213  ; v213 = 24
;; @0025                               store user2 v6, v23  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v42 = uadd_overflow_trap v19, v232, user2  ; v232 = 40
;;                                     v161 = load.i32 notrap v220
;;                                     v206 = iconst.i32 1
;; @0025                               v49 = band v161, v206  ; v206 = 1
;; @0025                               v24 = iconst.i32 0
;; @0025                               v50 = icmp eq v161, v24  ; v24 = 0
;; @0025                               v51 = uextend.i32 v50
;; @0025                               v52 = bor v49, v51
;; @0025                               brif v52, block3, block2
;;
;;                                 block2:
;;                                     v159 = load.i32 notrap v220
;; @0025                               v53 = uextend.i64 v159
;; @0025                               v55 = iadd.i64 v20, v53
;; @0025                               v56 = iconst.i64 8
;; @0025                               v57 = iadd v55, v56  ; v56 = 8
;; @0025                               v58 = load.i64 user2 v57
;;                                     v200 = iconst.i64 1
;; @0025                               v59 = iadd v58, v200  ; v200 = 1
;; @0025                               store user2 v59, v57
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v157 = load.i32 notrap v220
;; @0025                               v43 = uextend.i64 v42
;; @0025                               v45 = iadd.i64 v20, v43
;;                                     v222 = iconst.i64 12
;; @0025                               v48 = isub v45, v222  ; v222 = 12
;; @0025                               store user2 little v157, v48
;;                                     v325 = iadd.i64 v22, v213  ; v213 = 24
;; @0025                               v71 = load.i32 user2 readonly v325
;;                                     v326 = iconst.i32 1
;;                                     v327 = icmp ugt v71, v326  ; v326 = 1
;; @0025                               trapz v327, user17
;; @0025                               v74 = uextend.i64 v71
;;                                     v223 = iconst.i64 2
;;                                     v267 = ishl v74, v223  ; v223 = 2
;;                                     v216 = iconst.i64 32
;; @0025                               v76 = ushr v267, v216  ; v216 = 32
;; @0025                               trapnz v76, user2
;;                                     v244 = iconst.i32 2
;;                                     v274 = ishl v71, v244  ; v244 = 2
;; @0025                               v7 = iconst.i32 28
;; @0025                               v79 = uadd_overflow_trap v274, v7, user2  ; v7 = 28
;; @0025                               v83 = uadd_overflow_trap.i32 v19, v79, user2
;;                                     v156 = load.i32 notrap v219
;;                                     v328 = band v156, v326  ; v326 = 1
;;                                     v329 = iconst.i32 0
;;                                     v330 = icmp eq v156, v329  ; v329 = 0
;; @0025                               v92 = uextend.i32 v330
;; @0025                               v93 = bor v328, v92
;; @0025                               brif v93, block5, block4
;;
;;                                 block4:
;;                                     v154 = load.i32 notrap v219
;; @0025                               v94 = uextend.i64 v154
;; @0025                               v96 = iadd.i64 v20, v94
;;                                     v331 = iconst.i64 8
;; @0025                               v98 = iadd v96, v331  ; v331 = 8
;; @0025                               v99 = load.i64 user2 v98
;;                                     v332 = iconst.i64 1
;; @0025                               v100 = iadd v99, v332  ; v332 = 1
;; @0025                               store user2 v100, v98
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v152 = load.i32 notrap v219
;; @0025                               v84 = uextend.i64 v83
;; @0025                               v86 = iadd.i64 v20, v84
;;                                     v287 = iconst.i32 32
;; @0025                               v87 = isub.i32 v79, v287  ; v287 = 32
;; @0025                               v88 = uextend.i64 v87
;; @0025                               v89 = isub v86, v88
;; @0025                               store user2 little v152, v89
;;                                     v333 = iadd.i64 v22, v213  ; v213 = 24
;; @0025                               v112 = load.i32 user2 readonly v333
;;                                     v334 = iconst.i32 2
;;                                     v335 = icmp ugt v112, v334  ; v334 = 2
;; @0025                               trapz v335, user17
;; @0025                               v115 = uextend.i64 v112
;;                                     v336 = iconst.i64 2
;;                                     v337 = ishl v115, v336  ; v336 = 2
;;                                     v338 = iconst.i64 32
;;                                     v339 = ushr v337, v338  ; v338 = 32
;; @0025                               trapnz v339, user2
;;                                     v340 = ishl v112, v334  ; v334 = 2
;;                                     v341 = iconst.i32 28
;; @0025                               v120 = uadd_overflow_trap v340, v341, user2  ; v341 = 28
;; @0025                               v124 = uadd_overflow_trap.i32 v19, v120, user2
;;                                     v151 = load.i32 notrap v218
;;                                     v342 = iconst.i32 1
;;                                     v343 = band v151, v342  ; v342 = 1
;;                                     v344 = iconst.i32 0
;;                                     v345 = icmp eq v151, v344  ; v344 = 0
;; @0025                               v133 = uextend.i32 v345
;; @0025                               v134 = bor v343, v133
;; @0025                               brif v134, block7, block6
;;
;;                                 block6:
;;                                     v149 = load.i32 notrap v218
;; @0025                               v135 = uextend.i64 v149
;; @0025                               v137 = iadd.i64 v20, v135
;;                                     v346 = iconst.i64 8
;; @0025                               v139 = iadd v137, v346  ; v346 = 8
;; @0025                               v140 = load.i64 user2 v139
;;                                     v347 = iconst.i64 1
;; @0025                               v141 = iadd v140, v347  ; v347 = 1
;; @0025                               store user2 v141, v139
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v147 = load.i32 notrap v218
;; @0025                               v125 = uextend.i64 v124
;; @0025                               v127 = iadd.i64 v20, v125
;;                                     v319 = iconst.i32 36
;; @0025                               v128 = isub.i32 v120, v319  ; v319 = 36
;; @0025                               v129 = uextend.i64 v128
;; @0025                               v130 = isub v127, v129
;; @0025                               store user2 little v147, v130
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
