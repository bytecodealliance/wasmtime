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
;;                                     v213 = iconst.i64 24
;; @0025                               v23 = iadd v22, v213  ; v213 = 24
;; @0025                               store user2 region0 v6, v23  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v42 = uadd_overflow_trap v19, v232, user2  ; v232 = 40
;;                                     v164 = load.i32 notrap v220
;;                                     v206 = iconst.i32 1
;; @0025                               v49 = band v164, v206  ; v206 = 1
;; @0025                               v24 = iconst.i32 0
;; @0025                               v51 = icmp eq v164, v24  ; v24 = 0
;; @0025                               v52 = uextend.i32 v51
;; @0025                               v53 = bor v49, v52
;; @0025                               brif v53, block3, block2
;;
;;                                 block2:
;;                                     v162 = load.i32 notrap v220
;; @0025                               v54 = uextend.i64 v162
;; @0025                               v56 = iadd.i64 v20, v54
;; @0025                               v57 = iconst.i64 8
;; @0025                               v58 = iadd v56, v57  ; v57 = 8
;; @0025                               v59 = load.i64 user2 region0 v58
;;                                     v201 = iconst.i64 1
;; @0025                               v60 = iadd v59, v201  ; v201 = 1
;; @0025                               store user2 region0 v60, v58
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v160 = load.i32 notrap v220
;; @0025                               v43 = uextend.i64 v42
;; @0025                               v45 = iadd.i64 v20, v43
;;                                     v222 = iconst.i64 12
;; @0025                               v48 = isub v45, v222  ; v222 = 12
;; @0025                               store user2 little region0 v160, v48
;;                                     v325 = iadd.i64 v22, v213  ; v213 = 24
;; @0025                               v72 = load.i32 user2 readonly region0 v325
;;                                     v326 = iconst.i32 1
;;                                     v327 = icmp ugt v72, v326  ; v326 = 1
;; @0025                               trapz v327, user17
;; @0025                               v75 = uextend.i64 v72
;;                                     v223 = iconst.i64 2
;;                                     v267 = ishl v75, v223  ; v223 = 2
;;                                     v216 = iconst.i64 32
;; @0025                               v77 = ushr v267, v216  ; v216 = 32
;; @0025                               trapnz v77, user2
;;                                     v244 = iconst.i32 2
;;                                     v274 = ishl v72, v244  ; v244 = 2
;; @0025                               v7 = iconst.i32 28
;; @0025                               v80 = uadd_overflow_trap v274, v7, user2  ; v7 = 28
;; @0025                               v84 = uadd_overflow_trap.i32 v19, v80, user2
;;                                     v159 = load.i32 notrap v219
;;                                     v328 = band v159, v326  ; v326 = 1
;;                                     v329 = iconst.i32 0
;;                                     v330 = icmp eq v159, v329  ; v329 = 0
;; @0025                               v94 = uextend.i32 v330
;; @0025                               v95 = bor v328, v94
;; @0025                               brif v95, block5, block4
;;
;;                                 block4:
;;                                     v157 = load.i32 notrap v219
;; @0025                               v96 = uextend.i64 v157
;; @0025                               v98 = iadd.i64 v20, v96
;;                                     v331 = iconst.i64 8
;; @0025                               v100 = iadd v98, v331  ; v331 = 8
;; @0025                               v101 = load.i64 user2 region0 v100
;;                                     v332 = iconst.i64 1
;; @0025                               v102 = iadd v101, v332  ; v332 = 1
;; @0025                               store user2 region0 v102, v100
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v155 = load.i32 notrap v219
;; @0025                               v85 = uextend.i64 v84
;; @0025                               v87 = iadd.i64 v20, v85
;;                                     v287 = iconst.i32 32
;; @0025                               v88 = isub.i32 v80, v287  ; v287 = 32
;; @0025                               v89 = uextend.i64 v88
;; @0025                               v90 = isub v87, v89
;; @0025                               store user2 little region0 v155, v90
;;                                     v333 = iadd.i64 v22, v213  ; v213 = 24
;; @0025                               v114 = load.i32 user2 readonly region0 v333
;;                                     v334 = iconst.i32 2
;;                                     v335 = icmp ugt v114, v334  ; v334 = 2
;; @0025                               trapz v335, user17
;; @0025                               v117 = uextend.i64 v114
;;                                     v336 = iconst.i64 2
;;                                     v337 = ishl v117, v336  ; v336 = 2
;;                                     v338 = iconst.i64 32
;;                                     v339 = ushr v337, v338  ; v338 = 32
;; @0025                               trapnz v339, user2
;;                                     v340 = ishl v114, v334  ; v334 = 2
;;                                     v341 = iconst.i32 28
;; @0025                               v122 = uadd_overflow_trap v340, v341, user2  ; v341 = 28
;; @0025                               v126 = uadd_overflow_trap.i32 v19, v122, user2
;;                                     v154 = load.i32 notrap v218
;;                                     v342 = iconst.i32 1
;;                                     v343 = band v154, v342  ; v342 = 1
;;                                     v344 = iconst.i32 0
;;                                     v345 = icmp eq v154, v344  ; v344 = 0
;; @0025                               v136 = uextend.i32 v345
;; @0025                               v137 = bor v343, v136
;; @0025                               brif v137, block7, block6
;;
;;                                 block6:
;;                                     v152 = load.i32 notrap v218
;; @0025                               v138 = uextend.i64 v152
;; @0025                               v140 = iadd.i64 v20, v138
;;                                     v346 = iconst.i64 8
;; @0025                               v142 = iadd v140, v346  ; v346 = 8
;; @0025                               v143 = load.i64 user2 region0 v142
;;                                     v347 = iconst.i64 1
;; @0025                               v144 = iadd v143, v347  ; v347 = 1
;; @0025                               store user2 region0 v144, v142
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v150 = load.i32 notrap v218
;; @0025                               v127 = uextend.i64 v126
;; @0025                               v129 = iadd.i64 v20, v127
;;                                     v319 = iconst.i32 36
;; @0025                               v130 = isub.i32 v122, v319  ; v319 = 36
;; @0025                               v131 = uextend.i64 v130
;; @0025                               v132 = isub v129, v131
;; @0025                               store user2 little region0 v150, v132
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
