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
;;                                     v177 = stack_addr.i64 ss1
;;                                     store notrap v2, v177
;;                                     v176 = stack_addr.i64 ss0
;;                                     store notrap v4, v176
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;;                                     v174 = iconst.i64 1
;; @0020                               v9 = iadd v8, v174  ; v174 = 1
;; @0020                               v10 = iconst.i64 0
;; @0020                               v11 = icmp sge v9, v10  ; v10 = 0
;; @0020                               brif v11, block2, block3(v9)
;;
;;                                 block2:
;;                                     v186 = iadd.i64 v8, v174  ; v174 = 1
;; @0020                               store notrap aligned v186, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @0020                               v16 = load.i64 notrap aligned v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v73: i64):
;;                                     v128 = load.i32 notrap v177
;; @002b                               trapz v128, user16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v22 = uextend.i64 v128
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
;;                                     v125 = load.i32 notrap v176
;; @002b                               trapz v125, user16
;; @002b                               v41 = uextend.i64 v125
;; @002b                               v43 = iadd v23, v41
;; @002b                               v45 = iadd v43, v25  ; v25 = 16
;; @002b                               v46 = load.i32 user2 readonly v45
;; @002b                               v48 = uextend.i64 v5
;; @002b                               v51 = iadd v48, v30
;; @002b                               v47 = uextend.i64 v46
;; @002b                               v52 = icmp ugt v51, v47
;; @002b                               trapnz v52, user17
;; @002b                               v64 = load.i64 notrap aligned v7+40
;;                                     v163 = iconst.i64 20
;; @002b                               v37 = iadd v24, v163  ; v163 = 20
;;                                     v179 = iconst.i64 2
;;                                     v180 = ishl v29, v179  ; v179 = 2
;; @002b                               v40 = iadd v37, v180
;;                                     v184 = ishl v30, v179  ; v179 = 2
;; @002b                               v66 = uadd_overflow_trap v40, v184, user2
;; @002b                               v65 = iadd v23, v64
;; @002b                               v67 = icmp ugt v66, v65
;; @002b                               trapnz v67, user2
;; @002b                               v56 = iadd v43, v163  ; v163 = 20
;;                                     v182 = ishl v48, v179  ; v179 = 2
;; @002b                               v59 = iadd v56, v182
;; @002b                               v71 = uadd_overflow_trap v59, v184, user2
;; @002b                               v72 = icmp ugt v71, v65
;; @002b                               trapnz v72, user2
;;                                     v141 = iconst.i64 6
;; @002b                               v74 = iadd v73, v141  ; v141 = 6
;; @002b                               brif v30, block4, block7(v74)
;;
;;                                 block4:
;; @002b                               v75 = icmp.i64 ult v40, v59
;;                                     v187 = iadd.i64 v73, v141  ; v141 = 6
;; @002b                               v78 = iadd.i64 v40, v184
;; @002b                               v79 = iadd.i64 v59, v184
;; @002b                               v81 = iadd.i32 v5, v6
;;                                     v162 = iconst.i64 4
;; @002b                               v115 = iconst.i32 1
;; @002b                               brif v75, block5(v40, v59, v5, v187), block6(v78, v79, v81, v187)
;;
;;                                 block5(v82: i64, v83: i64, v84: i32, v85: i64):
;;                                     v197 = iconst.i64 1
;;                                     v198 = iadd v85, v197  ; v197 = 1
;;                                     v199 = iconst.i64 0
;;                                     v200 = icmp sge v198, v199  ; v199 = 0
;; @002b                               brif v200, block8, block9(v198)
;;
;;                                 block6(v99: i64, v100: i64, v101: i32, v102: i64):
;;                                     v188 = iconst.i64 1
;;                                     v189 = iadd v102, v188  ; v188 = 1
;;                                     v190 = iconst.i64 0
;;                                     v191 = icmp sge v189, v190  ; v190 = 0
;; @002b                               brif v191, block10, block11(v189)
;;
;;                                 block7(v122: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v198, v7
;; @002b                               v91 = call fn0(v0)
;; @002b                               v93 = load.i64 notrap aligned v7
;; @002b                               jump block9(v93)
;;
;;                                 block9(v119: i64):
;; @002b                               v94 = load.i32 user2 little v83
;; @002b                               store user2 little v94, v82
;;                                     v201 = iconst.i64 4
;;                                     v202 = iadd.i64 v83, v201  ; v201 = 4
;; @002b                               v98 = icmp eq v202, v79
;;                                     v203 = iadd.i64 v82, v201  ; v201 = 4
;;                                     v204 = iconst.i32 1
;;                                     v205 = iadd.i32 v84, v204  ; v204 = 1
;; @002b                               brif v98, block7(v119), block5(v203, v202, v205, v119)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v189, v7
;; @002b                               v108 = call fn0(v0)
;; @002b                               v110 = load.i64 notrap aligned v7
;; @002b                               jump block11(v110)
;;
;;                                 block11(v120: i64):
;;                                     v192 = iconst.i64 4
;;                                     v193 = isub.i64 v100, v192  ; v192 = 4
;; @002b                               v117 = load.i32 user2 little v193
;;                                     v194 = isub.i64 v99, v192  ; v192 = 4
;; @002b                               store user2 little v117, v194
;; @002b                               v118 = icmp eq v193, v59
;;                                     v195 = iconst.i32 1
;;                                     v196 = isub.i32 v101, v195  ; v195 = 1
;; @002b                               brif v118, block7(v120), block6(v194, v193, v196, v120)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v122, v7
;; @002f                               return
;; }
