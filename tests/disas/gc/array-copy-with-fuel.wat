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
;;     fn0 = colocated u805306368:11 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;;                                     v164 = stack_addr.i64 ss1
;;                                     store notrap v2, v164
;;                                     v163 = stack_addr.i64 ss0
;;                                     store notrap v4, v163
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;;                                     v161 = iconst.i64 1
;; @0020                               v9 = iadd v8, v161  ; v161 = 1
;; @0020                               v10 = iconst.i64 0
;; @0020                               v11 = icmp sge v9, v10  ; v10 = 0
;; @0020                               brif v11, block2, block3(v9)
;;
;;                                 block2:
;;                                     v176 = iadd.i64 v8, v161  ; v161 = 1
;; @0020                               store notrap aligned v176, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @0020                               v16 = load.i64 notrap aligned v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v113: i64):
;;                                     v123 = load.i32 notrap v164
;; @002b                               trapz v123, user16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v22 = uextend.i64 v123
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
;;                                     v120 = load.i32 notrap v163
;; @002b                               trapz v120, user16
;; @002b                               v41 = uextend.i64 v120
;; @002b                               v43 = iadd v23, v41
;; @002b                               v45 = iadd v43, v25  ; v25 = 16
;; @002b                               v46 = load.i32 user2 readonly v45
;; @002b                               v48 = uextend.i64 v5
;; @002b                               v51 = iadd v48, v30
;; @002b                               v47 = uextend.i64 v46
;; @002b                               v52 = icmp ugt v51, v47
;; @002b                               trapnz v52, user17
;; @002b                               v64 = load.i64 notrap aligned v7+40
;;                                     v150 = iconst.i64 20
;; @002b                               v37 = iadd v24, v150  ; v150 = 20
;;                                     v166 = iconst.i64 2
;;                                     v167 = ishl v29, v166  ; v166 = 2
;; @002b                               v40 = iadd v37, v167
;;                                     v171 = ishl v30, v166  ; v166 = 2
;; @002b                               v66 = uadd_overflow_trap v40, v171, user2
;; @002b                               v65 = iadd v23, v64
;; @002b                               v67 = icmp ugt v66, v65
;; @002b                               trapnz v67, user2
;; @002b                               brif v30, block4, block7(v113)
;;
;;                                 block4:
;;                                     v177 = iconst.i64 20
;;                                     v178 = iadd.i64 v43, v177  ; v177 = 20
;;                                     v179 = iconst.i64 2
;;                                     v180 = ishl.i64 v48, v179  ; v179 = 2
;; @002b                               v59 = iadd v178, v180
;; @002b                               v73 = icmp.i64 ult v40, v59
;; @002b                               v75 = iadd.i64 v40, v171
;; @002b                               v76 = iadd v59, v171
;; @002b                               v78 = iadd.i32 v5, v6
;;                                     v149 = iconst.i64 4
;; @002b                               v109 = iconst.i32 1
;;                                     v130 = iconst.i64 6
;; @002b                               brif v73, block5(v40, v59, v5, v113), block6(v75, v76, v78, v113)
;;
;;                                 block5(v79: i64, v80: i64, v81: i32, v82: i64):
;;                                     v188 = iconst.i64 6
;;                                     v189 = iadd v82, v188  ; v188 = 6
;;                                     v190 = iconst.i64 0
;;                                     v191 = icmp sge v189, v190  ; v190 = 0
;; @002b                               brif v191, block8, block9(v189)
;;
;;                                 block6(v96: i64, v97: i64, v98: i32, v100: i64):
;;                                     v181 = iconst.i64 0
;;                                     v182 = icmp sge v100, v181  ; v181 = 0
;; @002b                               brif v182, block10, block11(v100)
;;
;;                                 block7(v117: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v189, v7
;; @002b                               v88 = call fn0(v0)
;; @002b                               v90 = load.i64 notrap aligned v7
;; @002b                               jump block9(v90)
;;
;;                                 block9(v114: i64):
;; @002b                               v91 = load.i32 user2 little v80
;; @002b                               store user2 little v91, v79
;;                                     v192 = iconst.i64 4
;;                                     v193 = iadd.i64 v80, v192  ; v192 = 4
;; @002b                               v95 = icmp eq v193, v76
;;                                     v194 = iadd.i64 v79, v192  ; v192 = 4
;;                                     v195 = iconst.i32 1
;;                                     v196 = iadd.i32 v81, v195  ; v195 = 1
;; @002b                               brif v95, block7(v114), block5(v194, v193, v196, v114)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v100, v7
;; @002b                               v104 = call fn0(v0)
;; @002b                               v106 = load.i64 notrap aligned v7
;; @002b                               jump block11(v106)
;;
;;                                 block11(v115: i64):
;;                                     v183 = iconst.i64 4
;;                                     v184 = isub.i64 v97, v183  ; v183 = 4
;; @002b                               v111 = load.i32 user2 little v184
;;                                     v185 = isub.i64 v96, v183  ; v183 = 4
;; @002b                               store user2 little v111, v185
;; @002b                               v112 = icmp eq v184, v59
;;                                     v186 = iconst.i32 1
;;                                     v187 = isub.i32 v98, v186  ; v186 = 1
;; @002b                               brif v112, block7(v115), block6(v185, v184, v187, v115)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v117, v7
;; @002f                               return
;; }
