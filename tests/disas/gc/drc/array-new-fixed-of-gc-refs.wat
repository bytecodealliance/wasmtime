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
;;                                     v169 = load.i32 notrap v220
;;                                     v208 = iconst.i32 1
;; @0025                               v51 = band v169, v208  ; v208 = 1
;; @0025                               v26 = iconst.i32 0
;; @0025                               v53 = icmp eq v169, v26  ; v26 = 0
;; @0025                               v54 = uextend.i32 v53
;; @0025                               v55 = bor v51, v54
;; @0025                               brif v55, block3, block2
;;
;;                                 block2:
;;                                     v167 = load.i32 notrap v220
;; @0025                               v56 = uextend.i64 v167
;; @0025                               v58 = iadd.i64 v21, v56
;; @0025                               v59 = iconst.i64 8
;; @0025                               v60 = iadd v58, v59  ; v59 = 8
;; @0025                               v61 = load.i64 user2 region0 v60
;; @0025                               v62 = iconst.i64 1
;; @0025                               v63 = iadd v61, v62  ; v62 = 1
;; @0025                               store user2 region0 v63, v60
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v165 = load.i32 notrap v220
;; @0025                               v45 = uextend.i64 v44
;; @0025                               v47 = iadd.i64 v21, v45
;;                                     v222 = iconst.i64 12
;; @0025                               v50 = isub v47, v222  ; v222 = 12
;; @0025                               store user2 little region0 v165, v50
;;                                     v325 = iadd.i64 v23, v24  ; v24 = 24
;; @0025                               v75 = load.i32 user2 readonly region0 v325
;;                                     v326 = iconst.i32 1
;;                                     v327 = icmp ugt v75, v326  ; v326 = 1
;; @0025                               trapz v327, user17
;; @0025                               v78 = uextend.i64 v75
;;                                     v223 = iconst.i64 2
;;                                     v267 = ishl v78, v223  ; v223 = 2
;;                                     v217 = iconst.i64 32
;; @0025                               v80 = ushr v267, v217  ; v217 = 32
;; @0025                               trapnz v80, user2
;;                                     v244 = iconst.i32 2
;;                                     v274 = ishl v75, v244  ; v244 = 2
;; @0025                               v7 = iconst.i32 28
;; @0025                               v83 = uadd_overflow_trap v274, v7, user2  ; v7 = 28
;; @0025                               v87 = uadd_overflow_trap.i32 v20, v83, user2
;;                                     v164 = load.i32 notrap v219
;;                                     v328 = band v164, v326  ; v326 = 1
;;                                     v329 = iconst.i32 0
;;                                     v330 = icmp eq v164, v329  ; v329 = 0
;; @0025                               v97 = uextend.i32 v330
;; @0025                               v98 = bor v328, v97
;; @0025                               brif v98, block5, block4
;;
;;                                 block4:
;;                                     v162 = load.i32 notrap v219
;; @0025                               v99 = uextend.i64 v162
;; @0025                               v101 = iadd.i64 v21, v99
;;                                     v331 = iconst.i64 8
;; @0025                               v103 = iadd v101, v331  ; v331 = 8
;; @0025                               v104 = load.i64 user2 region0 v103
;;                                     v332 = iconst.i64 1
;; @0025                               v106 = iadd v104, v332  ; v332 = 1
;; @0025                               store user2 region0 v106, v103
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v160 = load.i32 notrap v219
;; @0025                               v88 = uextend.i64 v87
;; @0025                               v90 = iadd.i64 v21, v88
;;                                     v287 = iconst.i32 32
;; @0025                               v91 = isub.i32 v83, v287  ; v287 = 32
;; @0025                               v92 = uextend.i64 v91
;; @0025                               v93 = isub v90, v92
;; @0025                               store user2 little region0 v160, v93
;;                                     v333 = iadd.i64 v23, v24  ; v24 = 24
;; @0025                               v118 = load.i32 user2 readonly region0 v333
;;                                     v334 = iconst.i32 2
;;                                     v335 = icmp ugt v118, v334  ; v334 = 2
;; @0025                               trapz v335, user17
;; @0025                               v121 = uextend.i64 v118
;;                                     v336 = iconst.i64 2
;;                                     v337 = ishl v121, v336  ; v336 = 2
;;                                     v338 = iconst.i64 32
;;                                     v339 = ushr v337, v338  ; v338 = 32
;; @0025                               trapnz v339, user2
;;                                     v340 = ishl v118, v334  ; v334 = 2
;;                                     v341 = iconst.i32 28
;; @0025                               v126 = uadd_overflow_trap v340, v341, user2  ; v341 = 28
;; @0025                               v130 = uadd_overflow_trap.i32 v20, v126, user2
;;                                     v159 = load.i32 notrap v218
;;                                     v342 = iconst.i32 1
;;                                     v343 = band v159, v342  ; v342 = 1
;;                                     v344 = iconst.i32 0
;;                                     v345 = icmp eq v159, v344  ; v344 = 0
;; @0025                               v140 = uextend.i32 v345
;; @0025                               v141 = bor v343, v140
;; @0025                               brif v141, block7, block6
;;
;;                                 block6:
;;                                     v157 = load.i32 notrap v218
;; @0025                               v142 = uextend.i64 v157
;; @0025                               v144 = iadd.i64 v21, v142
;;                                     v346 = iconst.i64 8
;; @0025                               v146 = iadd v144, v346  ; v346 = 8
;; @0025                               v147 = load.i64 user2 region0 v146
;;                                     v347 = iconst.i64 1
;; @0025                               v149 = iadd v147, v347  ; v347 = 1
;; @0025                               store user2 region0 v149, v146
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v155 = load.i32 notrap v218
;; @0025                               v131 = uextend.i64 v130
;; @0025                               v133 = iadd.i64 v21, v131
;;                                     v319 = iconst.i32 36
;; @0025                               v134 = isub.i32 v126, v319  ; v319 = 36
;; @0025                               v135 = uextend.i64 v134
;; @0025                               v136 = isub v133, v135
;; @0025                               store user2 little region0 v155, v136
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v20
;; }
