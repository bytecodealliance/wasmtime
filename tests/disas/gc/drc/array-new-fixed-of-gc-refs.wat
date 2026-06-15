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
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 268435488 "VMStoreContext+0x20"
;;     region4 = 2147483648 "GcHeap"
;;     region5 = 268435496 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v203 = stack_addr.i64 ss2
;;                                     store notrap v2, v203
;;                                     v204 = stack_addr.i64 ss1
;;                                     store notrap v3, v204
;;                                     v205 = stack_addr.i64 ss0
;;                                     store notrap v4, v205
;; @0025                               v15 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v217 = iconst.i32 40
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v15, v17, v217, v18), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v15 = -1476395008, v217 = 40, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v20 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v21 = load.i64 notrap aligned readonly can_move region3 v20+32
;; @0025                               v22 = uextend.i64 v19
;; @0025                               v23 = iadd v21, v22
;; @0025                               v24 = iconst.i64 24
;; @0025                               v25 = iadd v23, v24  ; v24 = 24
;; @0025                               store user2 region4 v6, v25  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v46 = uadd_overflow_trap v19, v217, user2  ; v217 = 40
;;                                     v202 = load.i32 notrap v203
;; @0025                               v54 = iconst.i32 1
;; @0025                               v55 = band v202, v54  ; v54 = 1
;; @0025                               v26 = iconst.i32 0
;; @0025                               v57 = icmp eq v202, v26  ; v26 = 0
;; @0025                               v58 = uextend.i32 v57
;; @0025                               v59 = bor v55, v58
;; @0025                               brif v59, block3, block2
;;
;;                                 block2:
;;                                     v198 = load.i32 notrap v203
;; @0025                               v60 = uextend.i64 v198
;; @0025                               v63 = iadd.i64 v21, v60
;; @0025                               v64 = iconst.i64 8
;; @0025                               v65 = iadd v63, v64  ; v64 = 8
;; @0025                               v66 = load.i64 user2 region4 v65
;; @0025                               v67 = iconst.i64 1
;; @0025                               v68 = iadd v66, v67  ; v67 = 1
;; @0025                               store user2 region4 v68, v65
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v194 = load.i32 notrap v203
;; @0025                               v47 = uextend.i64 v46
;; @0025                               v50 = iadd.i64 v21, v47
;;                                     v207 = iconst.i64 12
;; @0025                               v53 = isub v50, v207  ; v207 = 12
;; @0025                               store user2 little region4 v194, v53
;;                                     v310 = iadd.i64 v23, v24  ; v24 = 24
;; @0025                               v82 = load.i32 user2 readonly region4 v310
;;                                     v311 = iconst.i32 1
;;                                     v312 = icmp ugt v82, v311  ; v311 = 1
;; @0025                               trapz v312, user17
;; @0025                               v85 = uextend.i64 v82
;;                                     v208 = iconst.i64 2
;;                                     v252 = ishl v85, v208  ; v208 = 2
;; @0025                               v11 = iconst.i64 32
;; @0025                               v88 = ushr v252, v11  ; v11 = 32
;; @0025                               trapnz v88, user2
;;                                     v229 = iconst.i32 2
;;                                     v259 = ishl v82, v229  ; v229 = 2
;; @0025                               v7 = iconst.i32 28
;; @0025                               v91 = uadd_overflow_trap v259, v7, user2  ; v7 = 28
;; @0025                               v95 = uadd_overflow_trap.i32 v19, v91, user2
;;                                     v192 = load.i32 notrap v204
;;                                     v313 = band v192, v311  ; v311 = 1
;;                                     v314 = iconst.i32 0
;;                                     v315 = icmp eq v192, v314  ; v314 = 0
;; @0025                               v107 = uextend.i32 v315
;; @0025                               v108 = bor v313, v107
;; @0025                               brif v108, block5, block4
;;
;;                                 block4:
;;                                     v188 = load.i32 notrap v204
;; @0025                               v109 = uextend.i64 v188
;; @0025                               v112 = iadd.i64 v21, v109
;;                                     v316 = iconst.i64 8
;; @0025                               v114 = iadd v112, v316  ; v316 = 8
;; @0025                               v115 = load.i64 user2 region4 v114
;;                                     v317 = iconst.i64 1
;; @0025                               v117 = iadd v115, v317  ; v317 = 1
;; @0025                               store user2 region4 v117, v114
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v184 = load.i32 notrap v204
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v99 = iadd.i64 v21, v96
;;                                     v272 = iconst.i32 32
;; @0025                               v100 = isub.i32 v91, v272  ; v272 = 32
;; @0025                               v101 = uextend.i64 v100
;; @0025                               v102 = isub v99, v101
;; @0025                               store user2 little region4 v184, v102
;;                                     v318 = iadd.i64 v23, v24  ; v24 = 24
;; @0025                               v131 = load.i32 user2 readonly region4 v318
;;                                     v319 = iconst.i32 2
;;                                     v320 = icmp ugt v131, v319  ; v319 = 2
;; @0025                               trapz v320, user17
;; @0025                               v134 = uextend.i64 v131
;;                                     v321 = iconst.i64 2
;;                                     v322 = ishl v134, v321  ; v321 = 2
;;                                     v323 = iconst.i64 32
;;                                     v324 = ushr v322, v323  ; v323 = 32
;; @0025                               trapnz v324, user2
;;                                     v325 = ishl v131, v319  ; v319 = 2
;;                                     v326 = iconst.i32 28
;; @0025                               v140 = uadd_overflow_trap v325, v326, user2  ; v326 = 28
;; @0025                               v144 = uadd_overflow_trap.i32 v19, v140, user2
;;                                     v182 = load.i32 notrap v205
;;                                     v327 = iconst.i32 1
;;                                     v328 = band v182, v327  ; v327 = 1
;;                                     v329 = iconst.i32 0
;;                                     v330 = icmp eq v182, v329  ; v329 = 0
;; @0025                               v156 = uextend.i32 v330
;; @0025                               v157 = bor v328, v156
;; @0025                               brif v157, block7, block6
;;
;;                                 block6:
;;                                     v178 = load.i32 notrap v205
;; @0025                               v158 = uextend.i64 v178
;; @0025                               v161 = iadd.i64 v21, v158
;;                                     v331 = iconst.i64 8
;; @0025                               v163 = iadd v161, v331  ; v331 = 8
;; @0025                               v164 = load.i64 user2 region4 v163
;;                                     v332 = iconst.i64 1
;; @0025                               v166 = iadd v164, v332  ; v332 = 1
;; @0025                               store user2 region4 v166, v163
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v174 = load.i32 notrap v205
;; @0025                               v145 = uextend.i64 v144
;; @0025                               v148 = iadd.i64 v21, v145
;;                                     v304 = iconst.i32 36
;; @0025                               v149 = isub.i32 v140, v304  ; v304 = 36
;; @0025                               v150 = uextend.i64 v149
;; @0025                               v151 = isub v148, v150
;; @0025                               store user2 little region4 v174, v151
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
