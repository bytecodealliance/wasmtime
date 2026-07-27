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
;;                                     v160 = stack_addr.i64 ss0
;;                                     store notrap aligned region6 v2, v160
;;                                     v161 = stack_addr.i64 ss1
;;                                     store notrap aligned region7 v4, v161
;; @0020                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v8 = load.i64 notrap aligned region2 v7
;; @0020                               v9 = iconst.i64 1
;; @0020                               v10 = iadd v8, v9  ; v9 = 1
;; @0020                               v11 = iconst.i64 0
;; @0020                               v12 = icmp sge v10, v11  ; v11 = 0
;; @0020                               brif v12, block2, block3(v10)
;;
;;                                 block2:
;;                                     v170 = iadd.i64 v8, v9  ; v9 = 1
;; @0020                               store notrap aligned region2 v170, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v16 = load.i64 notrap aligned region2 v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v23: i64):
;; @002b                               v24 = iconst.i64 6
;; @002b                               v25 = iadd v23, v24  ; v24 = 6
;; @002b                               v22 = uextend.i64 v6
;; @002b                               v26 = iadd v25, v22
;;                                     v171 = iconst.i64 0
;;                                     v172 = icmp sge v26, v171  ; v171 = 0
;; @002b                               brif v172, block4, block5(v26)
;;
;;                                 block4:
;; @002b                               store.i64 notrap aligned region2 v26, v7
;; @002b                               v30 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v32 = load.i64 notrap aligned region2 v7
;; @002b                               jump block5(v32)
;;
;;                                 block5(v137: i64):
;;                                     v159 = load.i32 notrap aligned region6 v160
;; @002b                               trapz v159, user16
;; @002b                               v35 = load.i64 notrap aligned readonly can_move region3 v7+32
;; @002b                               v33 = uextend.i64 v159
;; @002b                               v36 = iadd v35, v33
;; @002b                               v37 = iconst.i64 16
;; @002b                               v38 = iadd v36, v37  ; v37 = 16
;; @002b                               v39 = load.i32 user2 readonly region5 v38
;; @002b                               v41 = uextend.i64 v3
;; @002b                               v45 = iadd v41, v22
;; @002b                               v40 = uextend.i64 v39
;; @002b                               v46 = icmp ugt v45, v40
;; @002b                               trapnz v46, user17
;;                                     v153 = load.i32 notrap aligned region7 v161
;; @002b                               trapz v153, user16
;; @002b                               v57 = uextend.i64 v153
;; @002b                               v60 = iadd v35, v57
;; @002b                               v62 = iadd v60, v37  ; v37 = 16
;; @002b                               v63 = load.i32 user2 readonly region5 v62
;; @002b                               v65 = uextend.i64 v5
;; @002b                               v69 = iadd v65, v22
;; @002b                               v64 = uextend.i64 v63
;; @002b                               v70 = icmp ugt v69, v64
;; @002b                               trapnz v70, user17
;; @002b                               v89 = load.i64 notrap aligned region4 v7+40
;; @002b                               v51 = iconst.i64 20
;; @002b                               v52 = iadd v36, v51  ; v51 = 20
;;                                     v163 = iconst.i64 2
;;                                     v164 = ishl v41, v163  ; v163 = 2
;; @002b                               v56 = iadd v52, v164
;;                                     v168 = ishl.i64 v22, v163  ; v163 = 2
;; @002b                               v91 = uadd_overflow_trap v56, v168, user2
;; @002b                               v90 = iadd v35, v89
;; @002b                               v92 = icmp ugt v91, v90
;; @002b                               trapnz v92, user2
;; @002b                               v76 = iadd v60, v51  ; v51 = 20
;;                                     v166 = ishl v65, v163  ; v163 = 2
;; @002b                               v80 = iadd v76, v166
;; @002b                               v98 = uadd_overflow_trap v80, v168, user2
;; @002b                               v99 = icmp ugt v98, v90
;; @002b                               trapnz v99, user2
;; @002b                               brif.i32 v6, block6, block9
;;
;;                                 block6:
;;                                     v141 = load.i32 notrap aligned region6 v160
;;                                     v143 = load.i32 notrap aligned region7 v161
;; @002b                               v100 = icmp.i64 ult v56, v80
;; @002b                               v105 = iadd.i64 v56, v168
;; @002b                               v106 = iadd.i64 v80, v168
;; @002b                               v108 = iadd.i32 v5, v6
;; @002b                               v54 = iconst.i64 4
;; @002b                               v131 = iconst.i32 1
;; @002b                               brif v100, block7(v56, v80, v5), block8(v105, v106, v108)
;;
;;                                 block7(v109: i64, v110: i64, v111: i32):
;; @002b                               v114 = load.i32 user2 little region5 v110
;; @002b                               store user2 little region5 v114, v109
;;                                     v178 = iconst.i64 4
;;                                     v179 = iadd v110, v178  ; v178 = 4
;; @002b                               v121 = icmp eq v179, v106
;;                                     v180 = iadd v109, v178  ; v178 = 4
;;                                     v181 = iconst.i32 1
;;                                     v182 = iadd v111, v181  ; v181 = 1
;; @002b                               brif v121, block9, block7(v180, v179, v182)
;;
;;                                 block8(v122: i64, v123: i64, v124: i32):
;;                                     v173 = iconst.i64 4
;;                                     v174 = isub v123, v173  ; v173 = 4
;; @002b                               v133 = load.i32 user2 little region5 v174
;;                                     v175 = isub v122, v173  ; v173 = 4
;; @002b                               store user2 little region5 v133, v175
;; @002b                               v134 = icmp eq v174, v80
;;                                     v176 = iconst.i32 1
;;                                     v177 = isub v124, v176  ; v176 = 1
;; @002b                               brif v134, block9, block8(v175, v174, v177)
;;
;;                                 block9:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned region2 v137, v7
;; @002f                               return
;; }
