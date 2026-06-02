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
;;                                 block3(v76: i64):
;;                                     v149 = load.i32 notrap v201
;; @002b                               trapz v149, user16
;; @002b                               v24 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v23 = uextend.i64 v149
;; @002b                               v25 = iadd v24, v23
;; @002b                               v26 = iconst.i64 16
;; @002b                               v27 = iadd v25, v26  ; v26 = 16
;; @002b                               v28 = load.i32 user2 readonly region0 v27
;; @002b                               v30 = uextend.i64 v3
;; @002b                               v31 = uextend.i64 v6
;; @002b                               v33 = iadd v30, v31
;; @002b                               v29 = uextend.i64 v28
;; @002b                               v34 = icmp ugt v33, v29
;; @002b                               trapnz v34, user17
;;                                     v146 = load.i32 notrap v200
;; @002b                               trapz v146, user16
;; @002b                               v43 = uextend.i64 v146
;; @002b                               v45 = iadd v24, v43
;; @002b                               v47 = iadd v45, v26  ; v26 = 16
;; @002b                               v48 = load.i32 user2 readonly region0 v47
;; @002b                               v50 = uextend.i64 v5
;; @002b                               v53 = iadd v50, v31
;; @002b                               v49 = uextend.i64 v48
;; @002b                               v54 = icmp ugt v53, v49
;; @002b                               trapnz v54, user17
;; @002b                               v67 = load.i64 notrap aligned v7+40
;; @002b                               v38 = iconst.i64 20
;; @002b                               v39 = iadd v25, v38  ; v38 = 20
;;                                     v203 = iconst.i64 2
;;                                     v204 = ishl v30, v203  ; v203 = 2
;; @002b                               v42 = iadd v39, v204
;;                                     v208 = ishl v31, v203  ; v203 = 2
;; @002b                               v69 = uadd_overflow_trap v42, v208, user2
;; @002b                               v68 = iadd v24, v67
;; @002b                               v70 = icmp ugt v69, v68
;; @002b                               trapnz v70, user2
;; @002b                               v59 = iadd v45, v38  ; v38 = 20
;;                                     v206 = ishl v50, v203  ; v203 = 2
;; @002b                               v62 = iadd v59, v206
;; @002b                               v74 = uadd_overflow_trap v62, v208, user2
;; @002b                               v75 = icmp ugt v74, v68
;; @002b                               trapnz v75, user2
;; @002b                               v77 = iconst.i64 6
;; @002b                               v78 = iadd v76, v77  ; v77 = 6
;; @002b                               brif v31, block4, block7(v78)
;;
;;                                 block4:
;;                                     v140 = load.i32 notrap v201
;;                                     v141 = load.i32 notrap v200
;; @002b                               v79 = icmp.i64 ult v42, v62
;;                                     v211 = iadd.i64 v76, v77  ; v77 = 6
;; @002b                               v82 = iadd.i64 v42, v208
;; @002b                               v83 = iadd.i64 v62, v208
;; @002b                               v85 = iadd.i32 v5, v6
;;                                     v188 = iconst.i64 4
;; @002b                               v128 = iconst.i32 1
;; @002b                               brif v79, block5(v42, v62, v5, v140, v141, v211), block6(v82, v83, v85, v140, v141, v211)
;;
;;                                 block5(v86: i64, v87: i64, v88: i32, v89: i32, v90: i32, v91: i64):
;;                                     store notrap v89, v201
;;                                     store notrap v90, v200
;;                                     v221 = iconst.i64 1
;;                                     v222 = iadd v91, v221  ; v221 = 1
;;                                     v223 = iconst.i64 0
;;                                     v224 = icmp sge v222, v223  ; v223 = 0
;; @002b                               brif v224, block8, block9(v222)
;;
;;                                 block6(v109: i64, v110: i64, v111: i32, v112: i32, v113: i32, v114: i64):
;;                                     store notrap v112, v200
;;                                     store notrap v113, v201
;;                                     v212 = iconst.i64 1
;;                                     v213 = iadd v114, v212  ; v212 = 1
;;                                     v214 = iconst.i64 0
;;                                     v215 = icmp sge v213, v214  ; v214 = 0
;; @002b                               brif v215, block10, block11(v213)
;;
;;                                 block7(v135: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v222, v7
;; @002b                               v98 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v100 = load.i64 notrap aligned v7
;; @002b                               jump block9(v100)
;;
;;                                 block9(v132: i64):
;; @002b                               v101 = load.i32 user2 little region0 v87
;; @002b                               store user2 little region0 v101, v86
;;                                     v136 = load.i32 notrap v201
;;                                     v137 = load.i32 notrap v200
;;                                     v225 = iconst.i64 4
;;                                     v226 = iadd.i64 v87, v225  ; v225 = 4
;; @002b                               v108 = icmp eq v226, v83
;;                                     v227 = iadd.i64 v86, v225  ; v225 = 4
;;                                     v228 = iconst.i32 1
;;                                     v229 = iadd.i32 v88, v228  ; v228 = 1
;; @002b                               brif v108, block7(v132), block5(v227, v226, v229, v136, v137, v132)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v213, v7
;; @002b                               v121 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @002b                               v123 = load.i64 notrap aligned v7
;; @002b                               jump block11(v123)
;;
;;                                 block11(v133: i64):
;;                                     v216 = iconst.i64 4
;;                                     v217 = isub.i64 v110, v216  ; v216 = 4
;; @002b                               v130 = load.i32 user2 little region0 v217
;;                                     v218 = isub.i64 v109, v216  ; v216 = 4
;; @002b                               store user2 little region0 v130, v218
;;                                     v138 = load.i32 notrap v200
;;                                     v139 = load.i32 notrap v201
;; @002b                               v131 = icmp eq v217, v62
;;                                     v219 = iconst.i32 1
;;                                     v220 = isub.i32 v111, v219  ; v219 = 1
;; @002b                               brif v131, block7(v133), block6(v218, v217, v220, v138, v139, v133)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v135, v7
;; @002f                               return
;; }
