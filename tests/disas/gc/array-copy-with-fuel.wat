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
;;                                     v173 = stack_addr.i64 ss1
;;                                     store notrap v2, v173
;;                                     v172 = stack_addr.i64 ss0
;;                                     store notrap v4, v172
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;;                                     v170 = iconst.i64 1
;; @0020                               v9 = iadd v8, v170  ; v170 = 1
;; @0020                               v10 = iconst.i64 0
;; @0020                               v11 = icmp sge v9, v10  ; v10 = 0
;; @0020                               brif v11, block2, block3(v9)
;;
;;                                 block2:
;;                                     v182 = iadd.i64 v8, v170  ; v170 = 1
;; @0020                               store notrap aligned v182, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @0020                               v16 = load.i64 notrap aligned v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v116: i64):
;;                                     v126 = load.i32 notrap v173
;; @002b                               trapz v126, user16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v22 = uextend.i64 v126
;; @002b                               v24 = iadd v23, v22
;; @002b                               v25 = iconst.i64 16
;; @002b                               v26 = iadd v24, v25  ; v25 = 16
;; @002b                               v27 = load.i32 user2 readonly v26
;; @002b                               v29 = uextend.i64 v3
;; @002b                               v30 = uextend.i64 v6
;; @002b                               v32 = iadd v29, v30
;; @002b                               v28 = uextend.i64 v27
;; @002b                               v33 = icmp ugt v32, v28
;; @002b                               trapnz v33, user17
;;                                     v123 = load.i32 notrap v172
;; @002b                               trapz v123, user16
;; @002b                               v41 = uextend.i64 v123
;; @002b                               v43 = iadd v23, v41
;; @002b                               v45 = iadd v43, v25  ; v25 = 16
;; @002b                               v46 = load.i32 user2 readonly v45
;; @002b                               v48 = uextend.i64 v5
;; @002b                               v51 = iadd v48, v30
;; @002b                               v47 = uextend.i64 v46
;; @002b                               v52 = icmp ugt v51, v47
;; @002b                               trapnz v52, user17
;; @002b                               v64 = load.i64 notrap aligned v7+40
;;                                     v159 = iconst.i64 20
;; @002b                               v37 = iadd v24, v159  ; v159 = 20
;;                                     v175 = iconst.i64 2
;;                                     v176 = ishl v29, v175  ; v175 = 2
;; @002b                               v40 = iadd v37, v176
;;                                     v180 = ishl v30, v175  ; v175 = 2
;; @002b                               v66 = uadd_overflow_trap v40, v180, user2
;; @002b                               v65 = iadd v23, v64
;; @002b                               v67 = icmp ugt v66, v65
;; @002b                               trapnz v67, user2
;; @002b                               v56 = iadd v43, v159  ; v159 = 20
;;                                     v178 = ishl v48, v175  ; v175 = 2
;; @002b                               v59 = iadd v56, v178
;; @002b                               v71 = uadd_overflow_trap v59, v180, user2
;; @002b                               v72 = icmp ugt v71, v65
;; @002b                               trapnz v72, user2
;; @002b                               brif v30, block4, block7(v116)
;;
;;                                 block4:
;; @002b                               v73 = icmp.i64 ult v40, v59
;; @002b                               v76 = iadd.i64 v40, v180
;; @002b                               v77 = iadd.i64 v59, v180
;; @002b                               v79 = iadd.i32 v5, v6
;;                                     v158 = iconst.i64 4
;; @002b                               v112 = iconst.i32 1
;;                                     v135 = iconst.i64 6
;; @002b                               brif v73, block5(v40, v59, v5, v116), block6(v76, v77, v79, v116)
;;
;;                                 block5(v80: i64, v81: i64, v82: i32, v83: i64):
;;                                     v190 = iconst.i64 6
;;                                     v191 = iadd v83, v190  ; v190 = 6
;;                                     v192 = iconst.i64 0
;;                                     v193 = icmp sge v191, v192  ; v192 = 0
;; @002b                               brif v193, block8, block9(v191)
;;
;;                                 block6(v97: i64, v98: i64, v99: i32, v101: i64):
;;                                     v183 = iconst.i64 0
;;                                     v184 = icmp sge v101, v183  ; v183 = 0
;; @002b                               brif v184, block10, block11(v101)
;;
;;                                 block7(v120: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v191, v7
;; @002b                               v89 = call fn0(v0)
;; @002b                               v91 = load.i64 notrap aligned v7
;; @002b                               jump block9(v91)
;;
;;                                 block9(v117: i64):
;; @002b                               v92 = load.i32 user2 little v81
;; @002b                               store user2 little v92, v80
;;                                     v194 = iconst.i64 4
;;                                     v195 = iadd.i64 v81, v194  ; v194 = 4
;; @002b                               v96 = icmp eq v195, v77
;;                                     v196 = iadd.i64 v80, v194  ; v194 = 4
;;                                     v197 = iconst.i32 1
;;                                     v198 = iadd.i32 v82, v197  ; v197 = 1
;; @002b                               brif v96, block7(v117), block5(v196, v195, v198, v117)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v101, v7
;; @002b                               v105 = call fn0(v0)
;; @002b                               v107 = load.i64 notrap aligned v7
;; @002b                               jump block11(v107)
;;
;;                                 block11(v118: i64):
;;                                     v185 = iconst.i64 4
;;                                     v186 = isub.i64 v98, v185  ; v185 = 4
;; @002b                               v114 = load.i32 user2 little v186
;;                                     v187 = isub.i64 v97, v185  ; v185 = 4
;; @002b                               store user2 little v114, v187
;; @002b                               v115 = icmp eq v186, v59
;;                                     v188 = iconst.i32 1
;;                                     v189 = isub.i32 v99, v188  ; v188 = 1
;; @002b                               brif v115, block7(v118), block6(v187, v186, v189, v118)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v120, v7
;; @002f                               return
;; }
