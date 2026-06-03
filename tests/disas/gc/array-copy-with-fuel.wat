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
;;                                     v173 = stack_addr.i64 ss0
;;                                     store notrap v2, v173
;;                                     v174 = stack_addr.i64 ss1
;;                                     store notrap v4, v174
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;; @0020                               v9 = iconst.i64 1
;; @0020                               v10 = iadd v8, v9  ; v9 = 1
;; @0020                               v11 = iconst.i64 0
;; @0020                               v12 = icmp sge v10, v11  ; v11 = 0
;; @0020                               brif v12, block2, block3(v10)
;;
;;                                 block2:
;;                                     v207 = iadd.i64 v8, v9  ; v9 = 1
;; @0020                               store notrap aligned v207, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v16 = load.i64 notrap aligned v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v81: i64):
;;                                     v172 = load.i32 notrap v173
;; @002b                               trapz v172, user16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v22 = uextend.i64 v172
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
;;                                     v166 = load.i32 notrap v174
;; @002b                               trapz v166, user16
;; @002b                               v44 = uextend.i64 v166
;; @002b                               v46 = iadd v23, v44
;; @002b                               v48 = iadd v46, v25  ; v25 = 16
;; @002b                               v49 = load.i32 user2 readonly region0 v48
;; @002b                               v51 = uextend.i64 v5
;; @002b                               v55 = iadd v51, v30
;; @002b                               v50 = uextend.i64 v49
;; @002b                               v56 = icmp ugt v55, v50
;; @002b                               trapnz v56, user17
;; @002b                               v72 = load.i64 notrap aligned v7+40
;; @002b                               v38 = iconst.i64 20
;; @002b                               v39 = iadd v24, v38  ; v38 = 20
;;                                     v200 = iconst.i64 2
;;                                     v201 = ishl v29, v200  ; v200 = 2
;; @002b                               v43 = iadd v39, v201
;;                                     v205 = ishl v30, v200  ; v200 = 2
;; @002b                               v74 = uadd_overflow_trap v43, v205, user2
;; @002b                               v73 = iadd v23, v72
;; @002b                               v75 = icmp ugt v74, v73
;; @002b                               trapnz v75, user2
;; @002b                               v61 = iadd v46, v38  ; v38 = 20
;;                                     v203 = ishl v51, v200  ; v200 = 2
;; @002b                               v65 = iadd v61, v203
;; @002b                               v79 = uadd_overflow_trap v65, v205, user2
;; @002b                               v80 = icmp ugt v79, v73
;; @002b                               trapnz v80, user2
;; @002b                               v82 = iconst.i64 6
;; @002b                               v83 = iadd v81, v82  ; v82 = 6
;; @002b                               brif.i32 v6, block4, block7(v83)
;;
;;                                 block4:
;;                                     v154 = load.i32 notrap v173
;;                                     v156 = load.i32 notrap v174
;; @002b                               v84 = icmp.i64 ult v43, v65
;;                                     v208 = iadd.i64 v81, v82  ; v82 = 6
;; @002b                               v89 = iadd.i64 v43, v205
;; @002b                               v90 = iadd.i64 v65, v205
;; @002b                               v92 = iadd.i32 v5, v6
;; @002b                               v41 = iconst.i64 4
;; @002b                               v133 = iconst.i32 1
;; @002b                               brif v84, block5(v43, v65, v5, v154, v156, v208), block6(v89, v90, v92, v154, v156, v208)
;;
;;                                 block5(v93: i64, v94: i64, v95: i32, v96: i32, v97: i32, v98: i64):
;;                                     store notrap v96, v173
;;                                     store notrap v97, v174
;;                                     v218 = iconst.i64 1
;;                                     v219 = iadd v98, v218  ; v218 = 1
;;                                     v220 = iconst.i64 0
;;                                     v221 = icmp sge v219, v220  ; v220 = 0
;; @002b                               brif v221, block8, block9(v219)
;;
;;                                 block6(v115: i64, v116: i64, v117: i32, v118: i32, v119: i32, v120: i64):
;;                                     store notrap v118, v174
;;                                     store notrap v119, v173
;;                                     v209 = iconst.i64 1
;;                                     v210 = iadd v120, v209  ; v209 = 1
;;                                     v211 = iconst.i64 0
;;                                     v212 = icmp sge v210, v211  ; v211 = 0
;; @002b                               brif v212, block10, block11(v210)
;;
;;                                 block7(v140: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v219, v7
;; @002b                               v104 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v106 = load.i64 notrap aligned v7
;; @002b                               jump block9(v106)
;;
;;                                 block9(v137: i64):
;; @002b                               v107 = load.i32 user2 little region0 v94
;; @002b                               store user2 little region0 v107, v93
;;                                     v142 = load.i32 notrap v173
;;                                     v144 = load.i32 notrap v174
;;                                     v222 = iconst.i64 4
;;                                     v223 = iadd.i64 v94, v222  ; v222 = 4
;; @002b                               v114 = icmp eq v223, v90
;;                                     v224 = iadd.i64 v93, v222  ; v222 = 4
;;                                     v225 = iconst.i32 1
;;                                     v226 = iadd.i32 v95, v225  ; v225 = 1
;; @002b                               brif v114, block7(v137), block5(v224, v223, v226, v142, v144, v137)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v210, v7
;; @002b                               v126 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @002b                               v128 = load.i64 notrap aligned v7
;; @002b                               jump block11(v128)
;;
;;                                 block11(v138: i64):
;;                                     v213 = iconst.i64 4
;;                                     v214 = isub.i64 v116, v213  ; v213 = 4
;; @002b                               v135 = load.i32 user2 little region0 v214
;;                                     v215 = isub.i64 v115, v213  ; v213 = 4
;; @002b                               store user2 little region0 v135, v215
;;                                     v148 = load.i32 notrap v174
;;                                     v150 = load.i32 notrap v173
;; @002b                               v136 = icmp eq v214, v65
;;                                     v216 = iconst.i32 1
;;                                     v217 = isub.i32 v117, v216  ; v216 = 1
;; @002b                               brif v136, block7(v138), block6(v215, v214, v217, v148, v150, v138)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v140, v7
;; @002f                               return
;; }
