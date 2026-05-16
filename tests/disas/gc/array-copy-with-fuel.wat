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
;;                                     v155 = stack_addr.i64 ss0
;;                                     store notrap v2, v155
;;                                     v154 = stack_addr.i64 ss1
;;                                     store notrap v4, v154
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;;                                     v152 = iconst.i64 1
;; @0020                               v9 = iadd v8, v152  ; v152 = 1
;; @0020                               v10 = iconst.i64 0
;; @0020                               v11 = icmp sge v9, v10  ; v10 = 0
;; @0020                               brif v11, block2, block3(v9)
;;
;;                                 block2:
;;                                     v202 = iadd.i64 v8, v152  ; v152 = 1
;; @0020                               store notrap aligned v202, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v16 = load.i64 notrap aligned v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v116: i64):
;;                                     v126 = load.i32 notrap v154
;; @002b                               trapz v126, user16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v22 = uextend.i64 v126
;; @002b                               v24 = iadd v23, v22
;; @002b                               v25 = iconst.i64 16
;; @002b                               v26 = iadd v24, v25  ; v25 = 16
;; @002b                               v27 = load.i32 user2 readonly v26
;; @002b                               v28 = uadd_overflow_trap.i32 v5, v6, user17
;; @002b                               v29 = icmp ugt v28, v27
;; @002b                               trapnz v29, user17
;; @002b                               v31 = uextend.i64 v27
;;                                     v156 = iconst.i64 2
;;                                     v157 = ishl v31, v156  ; v156 = 2
;;                                     v145 = iconst.i64 32
;; @002b                               v33 = ushr v157, v145  ; v145 = 32
;; @002b                               trapnz v33, user2
;;                                     v166 = iconst.i32 2
;;                                     v167 = ishl v27, v166  ; v166 = 2
;; @002b                               v35 = iconst.i32 20
;; @002b                               v36 = uadd_overflow_trap v167, v35, user2  ; v35 = 20
;; @002b                               v40 = uadd_overflow_trap v126, v36, user2
;;                                     v123 = load.i32 notrap v155
;; @002b                               trapz v123, user16
;; @002b                               v47 = uextend.i64 v123
;; @002b                               v49 = iadd v23, v47
;; @002b                               v51 = iadd v49, v25  ; v25 = 16
;; @002b                               v52 = load.i32 user2 readonly v51
;; @002b                               v53 = uadd_overflow_trap.i32 v3, v6, user17
;; @002b                               v54 = icmp ugt v53, v52
;; @002b                               trapnz v54, user17
;; @002b                               v56 = uextend.i64 v52
;;                                     v177 = ishl v56, v156  ; v156 = 2
;; @002b                               v58 = ushr v177, v145  ; v145 = 32
;; @002b                               trapnz v58, user2
;;                                     v184 = ishl v52, v166  ; v166 = 2
;; @002b                               v61 = uadd_overflow_trap v184, v35, user2  ; v35 = 20
;; @002b                               v65 = uadd_overflow_trap v123, v61, user2
;; @002b                               v75 = uextend.i64 v6
;; @002b                               brif v75, block4, block7(v116)
;;
;;                                 block4:
;; @002b                               v66 = uextend.i64 v65
;; @002b                               v68 = iadd.i64 v23, v66
;;                                     v203 = iconst.i32 2
;;                                     v204 = ishl.i32 v3, v203  ; v203 = 2
;;                                     v205 = iconst.i32 20
;;                                     v206 = iadd v204, v205  ; v205 = 20
;; @002b                               v69 = isub.i32 v61, v206
;; @002b                               v70 = uextend.i64 v69
;; @002b                               v71 = isub v68, v70
;; @002b                               v41 = uextend.i64 v40
;; @002b                               v43 = iadd.i64 v23, v41
;;                                     v207 = ishl.i32 v5, v203  ; v203 = 2
;;                                     v208 = iadd v207, v205  ; v205 = 20
;; @002b                               v44 = isub.i32 v36, v208
;; @002b                               v45 = uextend.i64 v44
;; @002b                               v46 = isub v43, v45
;; @002b                               v76 = icmp ult v71, v46
;;                                     v209 = iconst.i64 2
;;                                     v210 = ishl.i64 v75, v209  ; v209 = 2
;; @002b                               v78 = iadd v71, v210
;; @002b                               v79 = iadd v46, v210
;; @002b                               v81 = iadd.i32 v5, v6
;; @002b                               v30 = iconst.i64 4
;;                                     v199 = iadd v43, v30  ; v30 = 4
;; @002b                               v112 = iconst.i32 1
;;                                     v133 = iconst.i64 6
;; @002b                               brif v76, block5(v71, v46, v5, v116), block6(v78, v79, v81, v116)
;;
;;                                 block5(v82: i64, v83: i64, v84: i32, v85: i64):
;;                                     v220 = iconst.i64 6
;;                                     v221 = iadd v85, v220  ; v220 = 6
;;                                     v222 = iconst.i64 0
;;                                     v223 = icmp sge v221, v222  ; v222 = 0
;; @002b                               brif v223, block8, block9(v221)
;;
;;                                 block6(v99: i64, v100: i64, v101: i32, v103: i64):
;;                                     v211 = iconst.i64 0
;;                                     v212 = icmp sge v103, v211  ; v211 = 0
;; @002b                               brif v212, block10, block11(v103)
;;
;;                                 block7(v120: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v221, v7
;; @002b                               v91 = call fn0(v0)
;; @002b                               v93 = load.i64 notrap aligned v7
;; @002b                               jump block9(v93)
;;
;;                                 block9(v117: i64):
;; @002b                               v94 = load.i32 user2 little v83
;; @002b                               store user2 little v94, v82
;;                                     v224 = iconst.i64 4
;;                                     v225 = iadd.i64 v83, v224  ; v224 = 4
;; @002b                               v98 = icmp eq v225, v79
;;                                     v226 = iadd.i64 v82, v224  ; v224 = 4
;;                                     v227 = iconst.i32 1
;;                                     v228 = iadd.i32 v84, v227  ; v227 = 1
;; @002b                               brif v98, block7(v117), block5(v226, v225, v228, v117)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v103, v7
;; @002b                               v107 = call fn0(v0)
;; @002b                               v109 = load.i64 notrap aligned v7
;; @002b                               jump block11(v109)
;;
;;                                 block11(v118: i64):
;;                                     v213 = iconst.i64 4
;;                                     v214 = isub.i64 v100, v213  ; v213 = 4
;; @002b                               v114 = load.i32 user2 little v214
;;                                     v215 = isub.i64 v99, v213  ; v213 = 4
;; @002b                               store user2 little v114, v215
;;                                     v198 = iadd.i64 v100, v45
;;                                     v216 = iadd.i64 v43, v30  ; v30 = 4
;;                                     v217 = icmp eq v198, v216
;;                                     v218 = iconst.i32 1
;;                                     v219 = isub.i32 v101, v218  ; v218 = 1
;; @002b                               brif v217, block7(v118), block6(v215, v214, v219, v118)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v120, v7
;; @002f                               return
;; }
