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
;;                                     v190 = stack_addr.i64 ss2
;;                                     store notrap v2, v190
;;                                     v191 = stack_addr.i64 ss1
;;                                     store notrap v3, v191
;;                                     v192 = stack_addr.i64 ss0
;;                                     store notrap v4, v192
;; @0025                               v15 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v230 = iconst.i32 40
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v15, v17, v230, v18), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v15 = -1476395008, v230 = 40, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v217 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v20 = load.i64 notrap aligned readonly can_move v217+32
;; @0025                               v21 = uextend.i64 v19
;; @0025                               v22 = iadd v20, v21
;; @0025                               v23 = iconst.i64 24
;; @0025                               v24 = iadd v22, v23  ; v23 = 24
;; @0025                               store user2 region0 v6, v24  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v44 = uadd_overflow_trap v19, v230, user2  ; v230 = 40
;;                                     v189 = load.i32 notrap v190
;; @0025                               v51 = iconst.i32 1
;; @0025                               v52 = band v189, v51  ; v51 = 1
;; @0025                               v25 = iconst.i32 0
;; @0025                               v54 = icmp eq v189, v25  ; v25 = 0
;; @0025                               v55 = uextend.i32 v54
;; @0025                               v56 = bor v52, v55
;; @0025                               brif v56, block3, block2
;;
;;                                 block2:
;;                                     v185 = load.i32 notrap v190
;; @0025                               v57 = uextend.i64 v185
;; @0025                               v59 = iadd.i64 v20, v57
;; @0025                               v60 = iconst.i64 8
;; @0025                               v61 = iadd v59, v60  ; v60 = 8
;; @0025                               v62 = load.i64 user2 region0 v61
;; @0025                               v63 = iconst.i64 1
;; @0025                               v64 = iadd v62, v63  ; v63 = 1
;; @0025                               store user2 region0 v64, v61
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v181 = load.i32 notrap v190
;; @0025                               v45 = uextend.i64 v44
;; @0025                               v47 = iadd.i64 v20, v45
;;                                     v220 = iconst.i64 12
;; @0025                               v50 = isub v47, v220  ; v220 = 12
;; @0025                               store user2 little region0 v181, v50
;;                                     v323 = iadd.i64 v22, v23  ; v23 = 24
;; @0025                               v76 = load.i32 user2 readonly region0 v323
;;                                     v324 = iconst.i32 1
;;                                     v325 = icmp ugt v76, v324  ; v324 = 1
;; @0025                               trapz v325, user17
;; @0025                               v79 = uextend.i64 v76
;;                                     v221 = iconst.i64 2
;;                                     v265 = ishl v79, v221  ; v221 = 2
;; @0025                               v11 = iconst.i64 32
;; @0025                               v82 = ushr v265, v11  ; v11 = 32
;; @0025                               trapnz v82, user2
;;                                     v242 = iconst.i32 2
;;                                     v272 = ishl v76, v242  ; v242 = 2
;; @0025                               v7 = iconst.i32 28
;; @0025                               v85 = uadd_overflow_trap v272, v7, user2  ; v7 = 28
;; @0025                               v89 = uadd_overflow_trap.i32 v19, v85, user2
;;                                     v179 = load.i32 notrap v191
;;                                     v326 = band v179, v324  ; v324 = 1
;;                                     v327 = iconst.i32 0
;;                                     v328 = icmp eq v179, v327  ; v327 = 0
;; @0025                               v100 = uextend.i32 v328
;; @0025                               v101 = bor v326, v100
;; @0025                               brif v101, block5, block4
;;
;;                                 block4:
;;                                     v175 = load.i32 notrap v191
;; @0025                               v102 = uextend.i64 v175
;; @0025                               v104 = iadd.i64 v20, v102
;;                                     v329 = iconst.i64 8
;; @0025                               v106 = iadd v104, v329  ; v329 = 8
;; @0025                               v107 = load.i64 user2 region0 v106
;;                                     v330 = iconst.i64 1
;; @0025                               v109 = iadd v107, v330  ; v330 = 1
;; @0025                               store user2 region0 v109, v106
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v171 = load.i32 notrap v191
;; @0025                               v90 = uextend.i64 v89
;; @0025                               v92 = iadd.i64 v20, v90
;;                                     v285 = iconst.i32 32
;; @0025                               v93 = isub.i32 v85, v285  ; v285 = 32
;; @0025                               v94 = uextend.i64 v93
;; @0025                               v95 = isub v92, v94
;; @0025                               store user2 little region0 v171, v95
;;                                     v331 = iadd.i64 v22, v23  ; v23 = 24
;; @0025                               v121 = load.i32 user2 readonly region0 v331
;;                                     v332 = iconst.i32 2
;;                                     v333 = icmp ugt v121, v332  ; v332 = 2
;; @0025                               trapz v333, user17
;; @0025                               v124 = uextend.i64 v121
;;                                     v334 = iconst.i64 2
;;                                     v335 = ishl v124, v334  ; v334 = 2
;;                                     v336 = iconst.i64 32
;;                                     v337 = ushr v335, v336  ; v336 = 32
;; @0025                               trapnz v337, user2
;;                                     v338 = ishl v121, v332  ; v332 = 2
;;                                     v339 = iconst.i32 28
;; @0025                               v130 = uadd_overflow_trap v338, v339, user2  ; v339 = 28
;; @0025                               v134 = uadd_overflow_trap.i32 v19, v130, user2
;;                                     v169 = load.i32 notrap v192
;;                                     v340 = iconst.i32 1
;;                                     v341 = band v169, v340  ; v340 = 1
;;                                     v342 = iconst.i32 0
;;                                     v343 = icmp eq v169, v342  ; v342 = 0
;; @0025                               v145 = uextend.i32 v343
;; @0025                               v146 = bor v341, v145
;; @0025                               brif v146, block7, block6
;;
;;                                 block6:
;;                                     v165 = load.i32 notrap v192
;; @0025                               v147 = uextend.i64 v165
;; @0025                               v149 = iadd.i64 v20, v147
;;                                     v344 = iconst.i64 8
;; @0025                               v151 = iadd v149, v344  ; v344 = 8
;; @0025                               v152 = load.i64 user2 region0 v151
;;                                     v345 = iconst.i64 1
;; @0025                               v154 = iadd v152, v345  ; v345 = 1
;; @0025                               store user2 region0 v154, v151
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v161 = load.i32 notrap v192
;; @0025                               v135 = uextend.i64 v134
;; @0025                               v137 = iadd.i64 v20, v135
;;                                     v317 = iconst.i32 36
;; @0025                               v138 = isub.i32 v130, v317  ; v317 = 36
;; @0025                               v139 = uextend.i64 v138
;; @0025                               v140 = isub v137, v139
;; @0025                               store user2 little region0 v161, v140
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
