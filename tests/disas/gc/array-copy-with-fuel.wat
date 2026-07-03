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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108864 "VMStoreContext+0x0"
;;     region3 = 67108896 "VMStoreContext+0x20"
;;     region4 = 67108904 "VMStoreContext+0x28"
;;     region5 = 536870912 "GcHeap"
;;     region6 = 1543503872 "Stack(ss0)"
;;     region7 = 1543503873 "Stack(ss1)"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:12 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;;                                     v181 = stack_addr.i64 ss0
;;                                     store notrap aligned region6 v2, v181
;;                                     v182 = stack_addr.i64 ss1
;;                                     store notrap aligned region7 v4, v182
;; @0020                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v8 = load.i64 notrap aligned region2 v7
;; @0020                               v9 = iconst.i64 1
;; @0020                               v10 = iadd v8, v9  ; v9 = 1
;; @0020                               v11 = iconst.i64 0
;; @0020                               v12 = icmp sge v10, v11  ; v11 = 0
;; @0020                               brif v12, block2, block3(v10)
;;
;;                                 block2:
;;                                     v191 = iadd.i64 v8, v9  ; v9 = 1
;; @0020                               store notrap aligned region2 v191, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v16 = load.i64 notrap aligned region2 v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v89: i64):
;;                                     v180 = load.i32 notrap aligned region6 v181
;; @002b                               trapz v180, user16
;; @002b                               v24 = load.i64 notrap aligned readonly can_move region3 v7+32
;; @002b                               v22 = uextend.i64 v180
;; @002b                               v25 = iadd v24, v22
;; @002b                               v26 = iconst.i64 16
;; @002b                               v27 = iadd v25, v26  ; v26 = 16
;; @002b                               v28 = load.i32 user2 readonly region5 v27
;; @002b                               v30 = uextend.i64 v3
;; @002b                               v31 = uextend.i64 v6
;; @002b                               v34 = iadd v30, v31
;; @002b                               v29 = uextend.i64 v28
;; @002b                               v35 = icmp ugt v34, v29
;; @002b                               trapnz v35, user17
;;                                     v174 = load.i32 notrap aligned region7 v182
;; @002b                               trapz v174, user16
;; @002b                               v46 = uextend.i64 v174
;; @002b                               v49 = iadd v24, v46
;; @002b                               v51 = iadd v49, v26  ; v26 = 16
;; @002b                               v52 = load.i32 user2 readonly region5 v51
;; @002b                               v54 = uextend.i64 v5
;; @002b                               v58 = iadd v54, v31
;; @002b                               v53 = uextend.i64 v52
;; @002b                               v59 = icmp ugt v58, v53
;; @002b                               trapnz v59, user17
;; @002b                               v78 = load.i64 notrap aligned region4 v7+40
;; @002b                               v40 = iconst.i64 20
;; @002b                               v41 = iadd v25, v40  ; v40 = 20
;;                                     v184 = iconst.i64 2
;;                                     v185 = ishl v30, v184  ; v184 = 2
;; @002b                               v45 = iadd v41, v185
;;                                     v189 = ishl v31, v184  ; v184 = 2
;; @002b                               v80 = uadd_overflow_trap v45, v189, user2
;; @002b                               v79 = iadd v24, v78
;; @002b                               v81 = icmp ugt v80, v79
;; @002b                               trapnz v81, user2
;; @002b                               v65 = iadd v49, v40  ; v40 = 20
;;                                     v187 = ishl v54, v184  ; v184 = 2
;; @002b                               v69 = iadd v65, v187
;; @002b                               v87 = uadd_overflow_trap v69, v189, user2
;; @002b                               v88 = icmp ugt v87, v79
;; @002b                               trapnz v88, user2
;; @002b                               v90 = iconst.i64 6
;; @002b                               v91 = iadd v89, v90  ; v90 = 6
;; @002b                               brif.i32 v6, block4, block7(v91)
;;
;;                                 block4:
;;                                     v162 = load.i32 notrap aligned region6 v181
;;                                     v164 = load.i32 notrap aligned region7 v182
;; @002b                               v92 = icmp.i64 ult v45, v69
;;                                     v192 = iadd.i64 v89, v90  ; v90 = 6
;; @002b                               v97 = iadd.i64 v45, v189
;; @002b                               v98 = iadd.i64 v69, v189
;; @002b                               v100 = iadd.i32 v5, v6
;; @002b                               v43 = iconst.i64 4
;; @002b                               v141 = iconst.i32 1
;; @002b                               brif v92, block5(v45, v69, v5, v162, v164, v192), block6(v97, v98, v100, v162, v164, v192)
;;
;;                                 block5(v101: i64, v102: i64, v103: i32, v104: i32, v105: i32, v106: i64):
;;                                     store notrap aligned region6 v104, v181
;;                                     store notrap aligned region7 v105, v182
;;                                     v202 = iconst.i64 1
;;                                     v203 = iadd v106, v202  ; v202 = 1
;;                                     v204 = iconst.i64 0
;;                                     v205 = icmp sge v203, v204  ; v204 = 0
;; @002b                               brif v205, block8, block9(v203)
;;
;;                                 block6(v123: i64, v124: i64, v125: i32, v126: i32, v127: i32, v128: i64):
;;                                     store notrap aligned region7 v126, v182
;;                                     store notrap aligned region6 v127, v181
;;                                     v193 = iconst.i64 1
;;                                     v194 = iadd v128, v193  ; v193 = 1
;;                                     v195 = iconst.i64 0
;;                                     v196 = icmp sge v194, v195  ; v195 = 0
;; @002b                               brif v196, block10, block11(v194)
;;
;;                                 block7(v148: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned region2 v203, v7
;; @002b                               v112 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v114 = load.i64 notrap aligned region2 v7
;; @002b                               jump block9(v114)
;;
;;                                 block9(v145: i64):
;; @002b                               v115 = load.i32 user2 little region5 v102
;; @002b                               store user2 little region5 v115, v101
;;                                     v150 = load.i32 notrap aligned region6 v181
;;                                     v152 = load.i32 notrap aligned region7 v182
;;                                     v206 = iconst.i64 4
;;                                     v207 = iadd.i64 v102, v206  ; v206 = 4
;; @002b                               v122 = icmp eq v207, v98
;;                                     v208 = iadd.i64 v101, v206  ; v206 = 4
;;                                     v209 = iconst.i32 1
;;                                     v210 = iadd.i32 v103, v209  ; v209 = 1
;; @002b                               brif v122, block7(v145), block5(v208, v207, v210, v150, v152, v145)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned region2 v194, v7
;; @002b                               v134 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @002b                               v136 = load.i64 notrap aligned region2 v7
;; @002b                               jump block11(v136)
;;
;;                                 block11(v146: i64):
;;                                     v197 = iconst.i64 4
;;                                     v198 = isub.i64 v124, v197  ; v197 = 4
;; @002b                               v143 = load.i32 user2 little region5 v198
;;                                     v199 = isub.i64 v123, v197  ; v197 = 4
;; @002b                               store user2 little region5 v143, v199
;;                                     v156 = load.i32 notrap aligned region7 v182
;;                                     v158 = load.i32 notrap aligned region6 v181
;; @002b                               v144 = icmp eq v198, v69
;;                                     v200 = iconst.i32 1
;;                                     v201 = isub.i32 v125, v200  ; v200 = 1
;; @002b                               brif v144, block7(v146), block6(v199, v198, v201, v156, v158, v146)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned region2 v148, v7
;; @002f                               return
;; }
