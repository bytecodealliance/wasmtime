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
;;                                     v192 = stack_addr.i64 ss2
;;                                     store notrap v2, v192
;;                                     v193 = stack_addr.i64 ss1
;;                                     store notrap v3, v193
;;                                     v194 = stack_addr.i64 ss0
;;                                     store notrap v4, v194
;; @0025                               v16 = iconst.i32 -1476395008
;; @0025                               v18 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v19 = load.i32 notrap aligned readonly can_move v18
;;                                     v232 = iconst.i32 40
;; @0025                               v20 = iconst.i32 8
;; @0025                               v21 = call fn0(v0, v16, v19, v232, v20), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v16 = -1476395008, v232 = 40, v20 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v219 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v22 = load.i64 notrap aligned readonly can_move v219+32
;; @0025                               v23 = uextend.i64 v21
;; @0025                               v24 = iadd v22, v23
;; @0025                               v25 = iconst.i64 24
;; @0025                               v26 = iadd v24, v25  ; v25 = 24
;; @0025                               store user2 region0 v6, v26  ; v6 = 3
;; @0025                               trapz v21, user16
;; @0025                               v46 = uadd_overflow_trap v21, v232, user2  ; v232 = 40
;;                                     v191 = load.i32 notrap v192
;; @0025                               v53 = iconst.i32 1
;; @0025                               v54 = band v191, v53  ; v53 = 1
;; @0025                               v27 = iconst.i32 0
;; @0025                               v56 = icmp eq v191, v27  ; v27 = 0
;; @0025                               v57 = uextend.i32 v56
;; @0025                               v58 = bor v54, v57
;; @0025                               brif v58, block3, block2
;;
;;                                 block2:
;;                                     v187 = load.i32 notrap v192
;; @0025                               v59 = uextend.i64 v187
;; @0025                               v61 = iadd.i64 v22, v59
;; @0025                               v62 = iconst.i64 8
;; @0025                               v63 = iadd v61, v62  ; v62 = 8
;; @0025                               v64 = load.i64 user2 region0 v63
;; @0025                               v65 = iconst.i64 1
;; @0025                               v66 = iadd v64, v65  ; v65 = 1
;; @0025                               store user2 region0 v66, v63
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v183 = load.i32 notrap v192
;; @0025                               v47 = uextend.i64 v46
;; @0025                               v49 = iadd.i64 v22, v47
;;                                     v222 = iconst.i64 12
;; @0025                               v52 = isub v49, v222  ; v222 = 12
;; @0025                               store user2 little region0 v183, v52
;;                                     v325 = iadd.i64 v24, v25  ; v25 = 24
;; @0025                               v78 = load.i32 user2 readonly region0 v325
;;                                     v326 = iconst.i32 1
;;                                     v327 = icmp ugt v78, v326  ; v326 = 1
;; @0025                               trapz v327, user17
;; @0025                               v81 = uextend.i64 v78
;;                                     v223 = iconst.i64 2
;;                                     v267 = ishl v81, v223  ; v223 = 2
;; @0025                               v11 = iconst.i64 32
;; @0025                               v84 = ushr v267, v11  ; v11 = 32
;; @0025                               trapnz v84, user2
;;                                     v244 = iconst.i32 2
;;                                     v274 = ishl v78, v244  ; v244 = 2
;; @0025                               v7 = iconst.i32 28
;; @0025                               v87 = uadd_overflow_trap v274, v7, user2  ; v7 = 28
;; @0025                               v91 = uadd_overflow_trap.i32 v21, v87, user2
;;                                     v181 = load.i32 notrap v193
;;                                     v328 = band v181, v326  ; v326 = 1
;;                                     v329 = iconst.i32 0
;;                                     v330 = icmp eq v181, v329  ; v329 = 0
;; @0025                               v102 = uextend.i32 v330
;; @0025                               v103 = bor v328, v102
;; @0025                               brif v103, block5, block4
;;
;;                                 block4:
;;                                     v177 = load.i32 notrap v193
;; @0025                               v104 = uextend.i64 v177
;; @0025                               v106 = iadd.i64 v22, v104
;;                                     v331 = iconst.i64 8
;; @0025                               v108 = iadd v106, v331  ; v331 = 8
;; @0025                               v109 = load.i64 user2 region0 v108
;;                                     v332 = iconst.i64 1
;; @0025                               v111 = iadd v109, v332  ; v332 = 1
;; @0025                               store user2 region0 v111, v108
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v173 = load.i32 notrap v193
;; @0025                               v92 = uextend.i64 v91
;; @0025                               v94 = iadd.i64 v22, v92
;;                                     v287 = iconst.i32 32
;; @0025                               v95 = isub.i32 v87, v287  ; v287 = 32
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v97 = isub v94, v96
;; @0025                               store user2 little region0 v173, v97
;;                                     v333 = iadd.i64 v24, v25  ; v25 = 24
;; @0025                               v123 = load.i32 user2 readonly region0 v333
;;                                     v334 = iconst.i32 2
;;                                     v335 = icmp ugt v123, v334  ; v334 = 2
;; @0025                               trapz v335, user17
;; @0025                               v126 = uextend.i64 v123
;;                                     v336 = iconst.i64 2
;;                                     v337 = ishl v126, v336  ; v336 = 2
;;                                     v338 = iconst.i64 32
;;                                     v339 = ushr v337, v338  ; v338 = 32
;; @0025                               trapnz v339, user2
;;                                     v340 = ishl v123, v334  ; v334 = 2
;;                                     v341 = iconst.i32 28
;; @0025                               v132 = uadd_overflow_trap v340, v341, user2  ; v341 = 28
;; @0025                               v136 = uadd_overflow_trap.i32 v21, v132, user2
;;                                     v171 = load.i32 notrap v194
;;                                     v342 = iconst.i32 1
;;                                     v343 = band v171, v342  ; v342 = 1
;;                                     v344 = iconst.i32 0
;;                                     v345 = icmp eq v171, v344  ; v344 = 0
;; @0025                               v147 = uextend.i32 v345
;; @0025                               v148 = bor v343, v147
;; @0025                               brif v148, block7, block6
;;
;;                                 block6:
;;                                     v167 = load.i32 notrap v194
;; @0025                               v149 = uextend.i64 v167
;; @0025                               v151 = iadd.i64 v22, v149
;;                                     v346 = iconst.i64 8
;; @0025                               v153 = iadd v151, v346  ; v346 = 8
;; @0025                               v154 = load.i64 user2 region0 v153
;;                                     v347 = iconst.i64 1
;; @0025                               v156 = iadd v154, v347  ; v347 = 1
;; @0025                               store user2 region0 v156, v153
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v163 = load.i32 notrap v194
;; @0025                               v137 = uextend.i64 v136
;; @0025                               v139 = iadd.i64 v22, v137
;;                                     v319 = iconst.i32 36
;; @0025                               v140 = isub.i32 v132, v319  ; v319 = 36
;; @0025                               v141 = uextend.i64 v140
;; @0025                               v142 = isub v139, v141
;; @0025                               store user2 little region0 v163, v142
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v21
;; }
