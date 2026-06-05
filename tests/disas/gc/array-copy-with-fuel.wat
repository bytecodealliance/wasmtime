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
;;                                     v201 = stack_addr.i64 ss0
;;                                     store notrap v2, v201
;;                                     v200 = stack_addr.i64 ss1
;;                                     store notrap v4, v200
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;; @0020                               v9 = iconst.i64 1
;; @0020                               v10 = iadd v8, v9  ; v9 = 1
;; @0020                               v11 = iconst.i64 0
;; @0020                               v12 = icmp sge v10, v11  ; v11 = 0
;; @0020                               brif v12, block2, block3(v10)
;;
;;                                 block2:
;;                                     v210 = iadd.i64 v8, v9  ; v9 = 1
;; @0020                               store notrap aligned v210, v7
;; @0020                               v15 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v17 = load.i64 notrap aligned v7
;; @0020                               jump block3(v17)
;;
;;                                 block3(v82: i64):
;;                                     v157 = load.i32 notrap v201
;; @002b                               trapz v157, user16
;; @002b                               v24 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v23 = uextend.i64 v157
;; @002b                               v25 = iadd v24, v23
;; @002b                               v26 = iconst.i64 16
;; @002b                               v27 = iadd v25, v26  ; v26 = 16
;; @002b                               v28 = load.i32 user2 readonly region0 v27
;; @002b                               v30 = uextend.i64 v3
;; @002b                               v31 = uextend.i64 v6
;; @002b                               v34 = iadd v30, v31
;; @002b                               v29 = uextend.i64 v28
;; @002b                               v35 = icmp ugt v34, v29
;; @002b                               trapnz v35, user17
;;                                     v154 = load.i32 notrap v200
;; @002b                               trapz v154, user16
;; @002b                               v45 = uextend.i64 v154
;; @002b                               v47 = iadd v24, v45
;; @002b                               v49 = iadd v47, v26  ; v26 = 16
;; @002b                               v50 = load.i32 user2 readonly region0 v49
;; @002b                               v52 = uextend.i64 v5
;; @002b                               v56 = iadd v52, v31
;; @002b                               v51 = uextend.i64 v50
;; @002b                               v57 = icmp ugt v56, v51
;; @002b                               trapnz v57, user17
;; @002b                               v73 = load.i64 notrap aligned v7+40
;; @002b                               v39 = iconst.i64 20
;; @002b                               v40 = iadd v25, v39  ; v39 = 20
;;                                     v203 = iconst.i64 2
;;                                     v204 = ishl v30, v203  ; v203 = 2
;; @002b                               v44 = iadd v40, v204
;;                                     v208 = ishl v31, v203  ; v203 = 2
;; @002b                               v75 = uadd_overflow_trap v44, v208, user2
;; @002b                               v74 = iadd v24, v73
;; @002b                               v76 = icmp ugt v75, v74
;; @002b                               trapnz v76, user2
;; @002b                               v62 = iadd v47, v39  ; v39 = 20
;;                                     v206 = ishl v52, v203  ; v203 = 2
;; @002b                               v66 = iadd v62, v206
;; @002b                               v80 = uadd_overflow_trap v66, v208, user2
;; @002b                               v81 = icmp ugt v80, v74
;; @002b                               trapnz v81, user2
;; @002b                               v83 = iconst.i64 6
;; @002b                               v84 = iadd v82, v83  ; v83 = 6
;; @002b                               brif.i32 v6, block4, block7(v84)
;;
;;                                 block4:
;;                                     v148 = load.i32 notrap v201
;;                                     v149 = load.i32 notrap v200
;; @002b                               v85 = icmp.i64 ult v44, v66
;;                                     v211 = iadd.i64 v82, v83  ; v83 = 6
;; @002b                               v90 = iadd.i64 v44, v208
;; @002b                               v91 = iadd.i64 v66, v208
;; @002b                               v93 = iadd.i32 v5, v6
;; @002b                               v42 = iconst.i64 4
;; @002b                               v136 = iconst.i32 1
;; @002b                               brif v85, block5(v44, v66, v5, v148, v149, v211), block6(v90, v91, v93, v148, v149, v211)
;;
;;                                 block5(v94: i64, v95: i64, v96: i32, v97: i32, v98: i32, v99: i64):
;;                                     store notrap v97, v201
;;                                     store notrap v98, v200
;;                                     v221 = iconst.i64 1
;;                                     v222 = iadd v99, v221  ; v221 = 1
;;                                     v223 = iconst.i64 0
;;                                     v224 = icmp sge v222, v223  ; v223 = 0
;; @002b                               brif v224, block8, block9(v222)
;;
;;                                 block6(v117: i64, v118: i64, v119: i32, v120: i32, v121: i32, v122: i64):
;;                                     store notrap v120, v200
;;                                     store notrap v121, v201
;;                                     v212 = iconst.i64 1
;;                                     v213 = iadd v122, v212  ; v212 = 1
;;                                     v214 = iconst.i64 0
;;                                     v215 = icmp sge v213, v214  ; v214 = 0
;; @002b                               brif v215, block10, block11(v213)
;;
;;                                 block7(v143: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v222, v7
;; @002b                               v106 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v108 = load.i64 notrap aligned v7
;; @002b                               jump block9(v108)
;;
;;                                 block9(v140: i64):
;; @002b                               v109 = load.i32 user2 little region0 v95
;; @002b                               store user2 little region0 v109, v94
;;                                     v144 = load.i32 notrap v201
;;                                     v145 = load.i32 notrap v200
;;                                     v225 = iconst.i64 4
;;                                     v226 = iadd.i64 v95, v225  ; v225 = 4
;; @002b                               v116 = icmp eq v226, v91
;;                                     v227 = iadd.i64 v94, v225  ; v225 = 4
;;                                     v228 = iconst.i32 1
;;                                     v229 = iadd.i32 v96, v228  ; v228 = 1
;; @002b                               brif v116, block7(v140), block5(v227, v226, v229, v144, v145, v140)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v213, v7
;; @002b                               v129 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @002b                               v131 = load.i64 notrap aligned v7
;; @002b                               jump block11(v131)
;;
;;                                 block11(v141: i64):
;;                                     v216 = iconst.i64 4
;;                                     v217 = isub.i64 v118, v216  ; v216 = 4
;; @002b                               v138 = load.i32 user2 little region0 v217
;;                                     v218 = isub.i64 v117, v216  ; v216 = 4
;; @002b                               store user2 little region0 v138, v218
;;                                     v146 = load.i32 notrap v200
;;                                     v147 = load.i32 notrap v201
;; @002b                               v139 = icmp eq v217, v66
;;                                     v219 = iconst.i32 1
;;                                     v220 = isub.i32 v119, v219  ; v219 = 1
;; @002b                               brif v139, block7(v141), block6(v218, v217, v220, v146, v147, v141)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v143, v7
;; @002f                               return
;; }
