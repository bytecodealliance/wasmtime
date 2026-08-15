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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 1677721600 "TypeIdsArray+0x0"
;;     region4 = 67108896 "VMStoreContext+0x20"
;;     region5 = 536870912 "GcHeap"
;;     region6 = 67108904 "VMStoreContext+0x28"
;;     region7 = 1543503872 "Stack(ss0)"
;;     region8 = 1543503873 "Stack(ss1)"
;;     region9 = 1543503874 "Stack(ss2)"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v202 = stack_addr.i64 ss2
;;                                     store notrap aligned region9 v2, v202
;;                                     v203 = stack_addr.i64 ss1
;;                                     store notrap aligned region8 v3, v203
;;                                     v204 = stack_addr.i64 ss0
;;                                     store notrap aligned region7 v4, v204
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v15 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0025                               v16 = load.i32 notrap aligned readonly can_move region3 v15
;;                                     v216 = iconst.i32 40
;; @0025                               v17 = iconst.i32 8
;; @0025                               v18 = call fn0(v0, v14, v16, v216, v17), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v14 = -1476395008, v216 = 40, v17 = 8
;; @0025                               v5 = iconst.i32 3
;; @0025                               v19 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v20 = load.i64 notrap aligned readonly can_move region4 v19+32
;; @0025                               v21 = uextend.i64 v18
;; @0025                               v22 = iadd v20, v21
;; @0025                               v23 = iconst.i64 24
;; @0025                               v24 = iadd v22, v23  ; v23 = 24
;; @0025                               store user2 region5 v5, v24  ; v5 = 3
;; @0025                               trapz v18, user16
;; @0025                               v45 = uadd_overflow_trap v18, v216, user2  ; v216 = 40
;;                                     v201 = load.i32 notrap aligned region9 v202
;; @0025                               v53 = iconst.i32 1
;; @0025                               v54 = band v201, v53  ; v53 = 1
;; @0025                               v25 = iconst.i32 0
;; @0025                               v56 = icmp eq v201, v25  ; v25 = 0
;; @0025                               v57 = uextend.i32 v56
;; @0025                               v58 = bor v54, v57
;; @0025                               brif v58, block3, block2
;;
;;                                 block2:
;; @0025                               v59 = uextend.i64 v201
;; @0025                               v62 = iadd.i64 v20, v59
;; @0025                               v63 = iconst.i64 8
;; @0025                               v64 = iadd v62, v63  ; v63 = 8
;; @0025                               v65 = load.i64 user2 region5 v64
;; @0025                               v66 = iconst.i64 1
;; @0025                               v67 = iadd v65, v66  ; v66 = 1
;; @0025                               store user2 region5 v67, v64
;; @0025                               jump block3
;;
;;                                 block3:
;; @0025                               v46 = uextend.i64 v45
;; @0025                               v49 = iadd.i64 v20, v46
;;                                     v206 = iconst.i64 12
;; @0025                               v52 = isub v49, v206  ; v206 = 12
;; @0025                               store.i32 user2 little region5 v201, v52
;;                                     v306 = iadd.i64 v22, v23  ; v23 = 24
;; @0025                               v81 = load.i32 user2 readonly region5 v306
;;                                     v307 = iconst.i32 1
;;                                     v308 = icmp ugt v81, v307  ; v307 = 1
;; @0025                               trapz v308, user17
;; @0025                               v84 = uextend.i64 v81
;;                                     v207 = iconst.i64 2
;;                                     v253 = ishl v84, v207  ; v207 = 2
;; @0025                               v10 = iconst.i64 32
;; @0025                               v87 = ushr v253, v10  ; v10 = 32
;; @0025                               trapnz v87, user2
;;                                     v228 = iconst.i32 2
;;                                     v258 = ishl v81, v228  ; v228 = 2
;; @0025                               v6 = iconst.i32 28
;; @0025                               v90 = uadd_overflow_trap v258, v6, user2  ; v6 = 28
;; @0025                               v94 = uadd_overflow_trap.i32 v18, v90, user2
;;                                     v191 = load.i32 notrap aligned region8 v203
;;                                     v309 = band v191, v307  ; v307 = 1
;;                                     v310 = iconst.i32 0
;;                                     v311 = icmp eq v191, v310  ; v310 = 0
;; @0025                               v106 = uextend.i32 v311
;; @0025                               v107 = bor v309, v106
;; @0025                               brif v107, block5, block4
;;
;;                                 block4:
;; @0025                               v108 = uextend.i64 v191
;; @0025                               v111 = iadd.i64 v20, v108
;;                                     v312 = iconst.i64 8
;; @0025                               v113 = iadd v111, v312  ; v312 = 8
;; @0025                               v114 = load.i64 user2 region5 v113
;;                                     v313 = iconst.i64 1
;; @0025                               v116 = iadd v114, v313  ; v313 = 1
;; @0025                               store user2 region5 v116, v113
;; @0025                               jump block5
;;
;;                                 block5:
;; @0025                               v95 = uextend.i64 v94
;; @0025                               v98 = iadd.i64 v20, v95
;;                                     v270 = iconst.i32 32
;; @0025                               v99 = isub.i32 v90, v270  ; v270 = 32
;; @0025                               v100 = uextend.i64 v99
;; @0025                               v101 = isub v98, v100
;; @0025                               store.i32 user2 little region5 v191, v101
;;                                     v314 = iadd.i64 v22, v23  ; v23 = 24
;; @0025                               v130 = load.i32 user2 readonly region5 v314
;;                                     v315 = iconst.i32 2
;;                                     v316 = icmp ugt v130, v315  ; v315 = 2
;; @0025                               trapz v316, user17
;; @0025                               v133 = uextend.i64 v130
;;                                     v317 = iconst.i64 2
;;                                     v318 = ishl v133, v317  ; v317 = 2
;;                                     v319 = iconst.i64 32
;;                                     v320 = ushr v318, v319  ; v319 = 32
;; @0025                               trapnz v320, user2
;;                                     v321 = ishl v130, v315  ; v315 = 2
;;                                     v322 = iconst.i32 28
;; @0025                               v139 = uadd_overflow_trap v321, v322, user2  ; v322 = 28
;; @0025                               v143 = uadd_overflow_trap.i32 v18, v139, user2
;;                                     v181 = load.i32 notrap aligned region7 v204
;;                                     v323 = iconst.i32 1
;;                                     v324 = band v181, v323  ; v323 = 1
;;                                     v325 = iconst.i32 0
;;                                     v326 = icmp eq v181, v325  ; v325 = 0
;; @0025                               v155 = uextend.i32 v326
;; @0025                               v156 = bor v324, v155
;; @0025                               brif v156, block7, block6
;;
;;                                 block6:
;; @0025                               v157 = uextend.i64 v181
;; @0025                               v160 = iadd.i64 v20, v157
;;                                     v327 = iconst.i64 8
;; @0025                               v162 = iadd v160, v327  ; v327 = 8
;; @0025                               v163 = load.i64 user2 region5 v162
;;                                     v328 = iconst.i64 1
;; @0025                               v165 = iadd v163, v328  ; v328 = 1
;; @0025                               store user2 region5 v165, v162
;; @0025                               jump block7
;;
;;                                 block7:
;; @0025                               v144 = uextend.i64 v143
;; @0025                               v147 = iadd.i64 v20, v144
;;                                     v300 = iconst.i32 36
;; @0025                               v148 = isub.i32 v139, v300  ; v300 = 36
;; @0025                               v149 = uextend.i64 v148
;; @0025                               v150 = isub v147, v149
;; @0025                               store.i32 user2 little region5 v181, v150
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v18
;; }
