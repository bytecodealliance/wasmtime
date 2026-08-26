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
;;                                 block3(v25: i64):
;; @002b                               v26 = iconst.i64 6
;; @002b                               v27 = iadd v25, v26  ; v26 = 6
;; @002b                               v22 = uextend.i64 v6
;; @002b                               v28 = iadd v27, v22
;;                                     v173 = iconst.i64 0
;;                                     v174 = icmp sge v28, v173  ; v173 = 0
;; @002b                               brif v174, block4, block5(v28)
;;
;;                                 block4:
;; @002b                               store.i64 notrap aligned region2 v28, v7
;; @002b                               v32 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v34 = load.i64 notrap aligned region2 v7
;; @002b                               jump block5(v34)
;;
;;                                 block5(v139: i64):
;;                                     v161 = load.i32 notrap aligned region6 v162
;; @002b                               trapz v161, user16
;; @002b                               v37 = load.i64 notrap aligned readonly can_move region3 v7+32
;; @002b                               v35 = uextend.i64 v161
;; @002b                               v38 = iadd v37, v35
;; @002b                               v39 = iconst.i64 16
;; @002b                               v40 = iadd v38, v39  ; v39 = 16
;; @002b                               v41 = load.i32 user2 readonly region5 v40
;; @002b                               v43 = uextend.i64 v3
;; @002b                               v47 = iadd v43, v22
;; @002b                               v42 = uextend.i64 v41
;; @002b                               v48 = icmp ugt v47, v42
;; @002b                               trapnz v48, user17
;;                                     v155 = load.i32 notrap aligned region7 v163
;; @002b                               trapz v155, user16
;; @002b                               v59 = uextend.i64 v155
;; @002b                               v62 = iadd v37, v59
;; @002b                               v64 = iadd v62, v39  ; v39 = 16
;; @002b                               v65 = load.i32 user2 readonly region5 v64
;; @002b                               v67 = uextend.i64 v5
;; @002b                               v71 = iadd v67, v22
;; @002b                               v66 = uextend.i64 v65
;; @002b                               v72 = icmp ugt v71, v66
;; @002b                               trapnz v72, user17
;; @002b                               v91 = load.i64 notrap aligned region4 v7+40
;; @002b                               v53 = iconst.i64 20
;; @002b                               v54 = iadd v38, v53  ; v53 = 20
;;                                     v165 = iconst.i64 2
;;                                     v166 = ishl v43, v165  ; v165 = 2
;; @002b                               v58 = iadd v54, v166
;;                                     v170 = ishl.i64 v22, v165  ; v165 = 2
;; @002b                               v93 = uadd_overflow_trap v58, v170, user2
;; @002b                               v92 = iadd v37, v91
;; @002b                               v94 = icmp ugt v93, v92
;; @002b                               trapnz v94, user2
;; @002b                               v78 = iadd v62, v53  ; v53 = 20
;;                                     v168 = ishl v67, v165  ; v165 = 2
;; @002b                               v82 = iadd v78, v168
;; @002b                               v100 = uadd_overflow_trap v82, v170, user2
;; @002b                               v101 = icmp ugt v100, v92
;; @002b                               trapnz v101, user2
;; @002b                               brif.i32 v6, block6, block9
;;
;;                                 block6:
;; @002b                               v102 = icmp.i64 ult v58, v82
;; @002b                               v107 = iadd.i64 v58, v170
;; @002b                               v108 = iadd.i64 v82, v170
;; @002b                               v110 = iadd.i32 v5, v6
;; @002b                               v56 = iconst.i64 4
;; @002b                               v133 = iconst.i32 1
;; @002b                               brif v102, block7(v58, v82, v5), block8(v107, v108, v110)
;;
;;                                 block7(v111: i64, v112: i64, v113: i32):
;; @002b                               v116 = load.i32 user2 little region5 v112
;; @002b                               store user2 little region5 v116, v111
;;                                     v180 = iconst.i64 4
;;                                     v181 = iadd v112, v180  ; v180 = 4
;; @002b                               v123 = icmp eq v181, v108
;;                                     v182 = iadd v111, v180  ; v180 = 4
;;                                     v183 = iconst.i32 1
;;                                     v184 = iadd v113, v183  ; v183 = 1
;; @002b                               brif v123, block9, block7(v182, v181, v184)
;;
;;                                 block8(v124: i64, v125: i64, v126: i32):
;;                                     v175 = iconst.i64 4
;;                                     v176 = isub v125, v175  ; v175 = 4
;; @002b                               v135 = load.i32 user2 little region5 v176
;;                                     v177 = isub v124, v175  ; v175 = 4
;; @002b                               store user2 little region5 v135, v177
;; @002b                               v136 = icmp eq v176, v82
;;                                     v178 = iconst.i32 1
;;                                     v179 = isub v126, v178  ; v178 = 1
;; @002b                               brif v136, block9, block8(v177, v176, v179)
;;
;;                                 block9:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned region2 v139, v7
;; @002f                               return
;; }
