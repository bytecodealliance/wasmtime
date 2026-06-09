;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc -Wfuel=0 -Ccollector=copying'

(module
  (type $a (array (mut anyref)))

  (func $copy (param (ref $a) i32 (ref $a) i32 i32)
    (array.copy $a $a (local.get 0) (local.get 1) (local.get 2) (local.get 3) (local.get 4))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32, i32) tail {
;;     ss0 = explicit_slot 4, align = 4
;;     ss1 = explicit_slot 4, align = 4
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:12 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;;                                     v177 = stack_addr.i64 ss0
;;                                     store notrap v2, v177
;;                                     v178 = stack_addr.i64 ss1
;;                                     store notrap v4, v178
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;; @0020                               v9 = iconst.i64 1
;; @0020                               v10 = iadd v8, v9  ; v9 = 1
;; @0020                               v11 = iconst.i64 0
;; @0020                               v12 = icmp sge v10, v11  ; v11 = 0
;; @0020                               brif v12, block2, block3(v10)
;;
;;                                 block2:
;;                                     v195 = iadd.i64 v8, v9  ; v9 = 1
;; @0020                               store notrap aligned v195, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v16 = load.i64 notrap aligned v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v85: i64):
;;                                     v176 = load.i32 notrap v177
;; @002b                               trapz v176, user16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v22 = uextend.i64 v176
;; @002b                               v24 = iadd v23, v22
;; @002b                               v25 = iconst.i64 16
;; @002b                               v26 = iadd v24, v25  ; v25 = 16
;; @002b                               v27 = load.i32 user2 readonly region0 v26
;; @002b                               v29 = uextend.i64 v3
;; @002b                               v30 = uextend.i64 v6
;; @002b                               v33 = iadd v29, v30
;; @002b                               v28 = uextend.i64 v27
;; @002b                               v34 = icmp ugt v33, v28
;; @002b                               trapnz v34, user17
;;                                     v170 = load.i32 notrap v178
;; @002b                               trapz v170, user16
;; @002b                               v45 = uextend.i64 v170
;; @002b                               v47 = iadd v23, v45
;; @002b                               v49 = iadd v47, v25  ; v25 = 16
;; @002b                               v50 = load.i32 user2 readonly region0 v49
;; @002b                               v52 = uextend.i64 v5
;; @002b                               v56 = iadd v52, v30
;; @002b                               v51 = uextend.i64 v50
;; @002b                               v57 = icmp ugt v56, v51
;; @002b                               trapnz v57, user17
;; @002b                               v75 = load.i64 notrap aligned v7+40
;; @002b                               v39 = iconst.i64 20
;; @002b                               v40 = iadd v24, v39  ; v39 = 20
;;                                     v188 = iconst.i64 2
;;                                     v189 = ishl v29, v188  ; v188 = 2
;; @002b                               v44 = iadd v40, v189
;;                                     v193 = ishl v30, v188  ; v188 = 2
;; @002b                               v77 = uadd_overflow_trap v44, v193, user2
;; @002b                               v76 = iadd v23, v75
;; @002b                               v78 = icmp ugt v77, v76
;; @002b                               trapnz v78, user2
;; @002b                               v63 = iadd v47, v39  ; v39 = 20
;;                                     v191 = ishl v52, v188  ; v188 = 2
;; @002b                               v67 = iadd v63, v191
;; @002b                               v83 = uadd_overflow_trap v67, v193, user2
;; @002b                               v84 = icmp ugt v83, v76
;; @002b                               trapnz v84, user2
;; @002b                               v86 = iconst.i64 6
;; @002b                               v87 = iadd v85, v86  ; v86 = 6
;; @002b                               brif.i32 v6, block4, block7(v87)
;;
;;                                 block4:
;;                                     v158 = load.i32 notrap v177
;;                                     v160 = load.i32 notrap v178
;; @002b                               v88 = icmp.i64 ult v44, v67
;;                                     v196 = iadd.i64 v85, v86  ; v86 = 6
;; @002b                               v93 = iadd.i64 v44, v193
;; @002b                               v94 = iadd.i64 v67, v193
;; @002b                               v96 = iadd.i32 v5, v6
;; @002b                               v42 = iconst.i64 4
;; @002b                               v137 = iconst.i32 1
;; @002b                               brif v88, block5(v44, v67, v5, v158, v160, v196), block6(v93, v94, v96, v158, v160, v196)
;;
;;                                 block5(v97: i64, v98: i64, v99: i32, v100: i32, v101: i32, v102: i64):
;;                                     store notrap v100, v177
;;                                     store notrap v101, v178
;;                                     v206 = iconst.i64 1
;;                                     v207 = iadd v102, v206  ; v206 = 1
;;                                     v208 = iconst.i64 0
;;                                     v209 = icmp sge v207, v208  ; v208 = 0
;; @002b                               brif v209, block8, block9(v207)
;;
;;                                 block6(v119: i64, v120: i64, v121: i32, v122: i32, v123: i32, v124: i64):
;;                                     store notrap v122, v178
;;                                     store notrap v123, v177
;;                                     v197 = iconst.i64 1
;;                                     v198 = iadd v124, v197  ; v197 = 1
;;                                     v199 = iconst.i64 0
;;                                     v200 = icmp sge v198, v199  ; v199 = 0
;; @002b                               brif v200, block10, block11(v198)
;;
;;                                 block7(v144: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v207, v7
;; @002b                               v108 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v110 = load.i64 notrap aligned v7
;; @002b                               jump block9(v110)
;;
;;                                 block9(v141: i64):
;; @002b                               v111 = load.i32 user2 little region0 v98
;; @002b                               store user2 little region0 v111, v97
;;                                     v146 = load.i32 notrap v177
;;                                     v148 = load.i32 notrap v178
;;                                     v210 = iconst.i64 4
;;                                     v211 = iadd.i64 v98, v210  ; v210 = 4
;; @002b                               v118 = icmp eq v211, v94
;;                                     v212 = iadd.i64 v97, v210  ; v210 = 4
;;                                     v213 = iconst.i32 1
;;                                     v214 = iadd.i32 v99, v213  ; v213 = 1
;; @002b                               brif v118, block7(v141), block5(v212, v211, v214, v146, v148, v141)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v198, v7
;; @002b                               v130 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @002b                               v132 = load.i64 notrap aligned v7
;; @002b                               jump block11(v132)
;;
;;                                 block11(v142: i64):
;;                                     v201 = iconst.i64 4
;;                                     v202 = isub.i64 v120, v201  ; v201 = 4
;; @002b                               v139 = load.i32 user2 little region0 v202
;;                                     v203 = isub.i64 v119, v201  ; v201 = 4
;; @002b                               store user2 little region0 v139, v203
;;                                     v152 = load.i32 notrap v178
;;                                     v154 = load.i32 notrap v177
;; @002b                               v140 = icmp eq v202, v67
;;                                     v204 = iconst.i32 1
;;                                     v205 = isub.i32 v121, v204  ; v204 = 1
;; @002b                               brif v140, block7(v142), block6(v203, v202, v205, v152, v154, v142)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v144, v7
;; @002f                               return
;; }
