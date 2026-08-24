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
;;                                 block3(v22: i64):
;; @002b                               v23 = iconst.i64 6
;; @002b                               v24 = iadd v22, v23  ; v23 = 6
;;                                     v173 = iconst.i64 0
;;                                     v174 = icmp sge v24, v173  ; v173 = 0
;; @002b                               brif v174, block4, block5(v24)
;;
;;                                 block4:
;;                                     v175 = iadd.i64 v22, v23  ; v23 = 6
;; @002b                               store notrap aligned region2 v175, v7
;; @002b                               v28 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v30 = load.i64 notrap aligned region2 v7
;; @002b                               jump block5(v30)
;;
;;                                 block5(v134: i64):
;;                                     v161 = load.i32 notrap aligned region6 v162
;; @002b                               trapz v161, user16
;; @002b                               v33 = load.i64 notrap aligned readonly can_move region3 v7+32
;; @002b                               v31 = uextend.i64 v161
;; @002b                               v34 = iadd v33, v31
;; @002b                               v35 = iconst.i64 16
;; @002b                               v36 = iadd v34, v35  ; v35 = 16
;; @002b                               v37 = load.i32 user2 readonly region5 v36
;; @002b                               v39 = uextend.i64 v3
;; @002b                               v40 = uextend.i64 v6
;; @002b                               v43 = iadd v39, v40
;; @002b                               v38 = uextend.i64 v37
;; @002b                               v44 = icmp ugt v43, v38
;; @002b                               trapnz v44, user17
;;                                     v155 = load.i32 notrap aligned region7 v163
;; @002b                               trapz v155, user16
;; @002b                               v55 = uextend.i64 v155
;; @002b                               v58 = iadd v33, v55
;; @002b                               v60 = iadd v58, v35  ; v35 = 16
;; @002b                               v61 = load.i32 user2 readonly region5 v60
;; @002b                               v63 = uextend.i64 v5
;; @002b                               v67 = iadd v63, v40
;; @002b                               v62 = uextend.i64 v61
;; @002b                               v68 = icmp ugt v67, v62
;; @002b                               trapnz v68, user17
;; @002b                               v87 = load.i64 notrap aligned region4 v7+40
;; @002b                               v49 = iconst.i64 20
;; @002b                               v50 = iadd v34, v49  ; v49 = 20
;;                                     v165 = iconst.i64 2
;;                                     v166 = ishl v39, v165  ; v165 = 2
;; @002b                               v54 = iadd v50, v166
;;                                     v170 = ishl v40, v165  ; v165 = 2
;; @002b                               v89 = uadd_overflow_trap v54, v170, user2
;; @002b                               v88 = iadd v33, v87
;; @002b                               v90 = icmp ugt v89, v88
;; @002b                               trapnz v90, user2
;; @002b                               v74 = iadd v58, v49  ; v49 = 20
;;                                     v168 = ishl v63, v165  ; v165 = 2
;; @002b                               v78 = iadd v74, v168
;; @002b                               v96 = uadd_overflow_trap v78, v170, user2
;; @002b                               v97 = icmp ugt v96, v88
;; @002b                               trapnz v97, user2
;; @002b                               brif.i32 v6, block6, block9
;;
;;                                 block6:
;;                                     v143 = load.i32 notrap aligned region6 v162
;;                                     v145 = load.i32 notrap aligned region7 v163
;; @002b                               v98 = icmp.i64 ult v54, v78
;; @002b                               v103 = iadd.i64 v54, v170
;; @002b                               v104 = iadd.i64 v78, v170
;; @002b                               v106 = iadd.i32 v5, v6
;; @002b                               v52 = iconst.i64 4
;; @002b                               v129 = iconst.i32 1
;; @002b                               brif v98, block7(v54, v78, v5), block8(v103, v104, v106)
;;
;;                                 block7(v107: i64, v108: i64, v109: i32):
;; @002b                               v112 = load.i32 user2 little region5 v108
;; @002b                               store user2 little region5 v112, v107
;;                                     v181 = iconst.i64 4
;;                                     v182 = iadd v108, v181  ; v181 = 4
;; @002b                               v119 = icmp eq v182, v104
;;                                     v183 = iadd v107, v181  ; v181 = 4
;;                                     v184 = iconst.i32 1
;;                                     v185 = iadd v109, v184  ; v184 = 1
;; @002b                               brif v119, block9, block7(v183, v182, v185)
;;
;;                                 block8(v120: i64, v121: i64, v122: i32):
;;                                     v176 = iconst.i64 4
;;                                     v177 = isub v121, v176  ; v176 = 4
;; @002b                               v131 = load.i32 user2 little region5 v177
;;                                     v178 = isub v120, v176  ; v176 = 4
;; @002b                               store user2 little region5 v131, v178
;; @002b                               v132 = icmp eq v177, v78
;;                                     v179 = iconst.i32 1
;;                                     v180 = isub v122, v179  ; v179 = 1
;; @002b                               brif v132, block9, block8(v178, v177, v180)
;;
;;                                 block9:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002b                               v140 = iadd.i64 v134, v40
;; @002f                               store notrap aligned region2 v140, v7
;; @002f                               return
;; }
