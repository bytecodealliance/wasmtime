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
;;                                     v162 = stack_addr.i64 ss0
;;                                     store notrap aligned region6 v2, v162
;;                                     v163 = stack_addr.i64 ss1
;;                                     store notrap aligned region7 v4, v163
;; @0020                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v8 = load.i64 notrap aligned region2 v7
;; @0020                               v9 = iconst.i64 1
;; @0020                               v10 = iadd v8, v9  ; v9 = 1
;; @0020                               v11 = iconst.i64 0
;; @0020                               v12 = icmp sge v10, v11  ; v11 = 0
;; @0020                               brif v12, block2, block3(v10)
;;
;;                                 block2:
;;                                     v172 = iadd.i64 v8, v9  ; v9 = 1
;; @0020                               store notrap aligned region2 v172, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v16 = load.i64 notrap aligned region2 v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v125: i64):
;;                                     v161 = load.i32 notrap aligned region6 v162
;; @002b                               trapz v161, user16
;; @002b                               v24 = load.i64 notrap aligned readonly can_move region3 v7+32
;; @002b                               v22 = uextend.i64 v161
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
;;                                     v155 = load.i32 notrap aligned region7 v163
;; @002b                               trapz v155, user16
;; @002b                               v46 = uextend.i64 v155
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
;;                                     v165 = iconst.i64 2
;;                                     v166 = ishl v30, v165  ; v165 = 2
;; @002b                               v45 = iadd v41, v166
;;                                     v170 = ishl v31, v165  ; v165 = 2
;; @002b                               v80 = uadd_overflow_trap v45, v170, user2
;; @002b                               v79 = iadd v24, v78
;; @002b                               v81 = icmp ugt v80, v79
;; @002b                               trapnz v81, user2
;; @002b                               v65 = iadd v49, v40  ; v40 = 20
;;                                     v168 = ishl v54, v165  ; v165 = 2
;; @002b                               v69 = iadd v65, v168
;; @002b                               v87 = uadd_overflow_trap v69, v170, user2
;; @002b                               v88 = icmp ugt v87, v79
;; @002b                               trapnz v88, user2
;; @002b                               brif.i32 v6, block4, block7
;;
;;                                 block4:
;; @002b                               v89 = icmp.i64 ult v45, v69
;; @002b                               v94 = iadd.i64 v45, v170
;; @002b                               v95 = iadd.i64 v69, v170
;; @002b                               v97 = iadd.i32 v5, v6
;; @002b                               v43 = iconst.i64 4
;; @002b                               v120 = iconst.i32 1
;; @002b                               brif v89, block5(v45, v69, v5), block6(v94, v95, v97)
;;
;;                                 block5(v98: i64, v99: i64, v100: i32):
;; @002b                               v103 = load.i32 user2 little region5 v99
;; @002b                               store user2 little region5 v103, v98
;;                                     v178 = iconst.i64 4
;;                                     v179 = iadd v99, v178  ; v178 = 4
;; @002b                               v110 = icmp eq v179, v95
;;                                     v180 = iadd v98, v178  ; v178 = 4
;;                                     v181 = iconst.i32 1
;;                                     v182 = iadd v100, v181  ; v181 = 1
;; @002b                               brif v110, block7, block5(v180, v179, v182)
;;
;;                                 block6(v111: i64, v112: i64, v113: i32):
;;                                     v173 = iconst.i64 4
;;                                     v174 = isub v112, v173  ; v173 = 4
;; @002b                               v122 = load.i32 user2 little region5 v174
;;                                     v175 = isub v111, v173  ; v173 = 4
;; @002b                               store user2 little region5 v122, v175
;; @002b                               v123 = icmp eq v174, v69
;;                                     v176 = iconst.i32 1
;;                                     v177 = isub v113, v176  ; v176 = 1
;; @002b                               brif v123, block7, block6(v175, v174, v177)
;;
;;                                 block7:
;; @002b                               v128 = iconst.i64 6
;; @002b                               v129 = iadd.i64 v125, v128  ; v128 = 6
;; @002b                               v133 = iadd v129, v31
;;                                     v183 = iconst.i64 0
;;                                     v184 = icmp sge v133, v183  ; v183 = 0
;; @002b                               brif v184, block8, block9(v133)
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned region2 v133, v7
;; @002b                               v137 = call fn0(v0)
;; @002b                               v139 = load.i64 notrap aligned region2 v7
;; @002b                               jump block9(v139)
;;
;;                                 block9(v141: i64):
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned region2 v141, v7
;; @002f                               return
;; }
