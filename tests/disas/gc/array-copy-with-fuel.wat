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
;;                                     v177 = stack_addr.i64 ss0
;;                                     store notrap v2, v177
;;                                     v176 = stack_addr.i64 ss1
;;                                     store notrap v4, v176
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;; @0020                               v9 = iconst.i64 1
;; @0020                               v10 = iadd v8, v9  ; v9 = 1
;; @0020                               v11 = iconst.i64 0
;; @0020                               v12 = icmp sge v10, v11  ; v11 = 0
;; @0020                               brif v12, block2, block3(v10)
;;
;;                                 block2:
;;                                     v186 = iadd.i64 v8, v9  ; v9 = 1
;; @0020                               store notrap aligned v186, v7
;; @0020                               v15 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v17 = load.i64 notrap aligned v7
;; @0020                               jump block3(v17)
;;
;;                                 block3(v24: i64):
;; @002b                               v25 = iconst.i64 6
;; @002b                               v26 = iadd v24, v25  ; v25 = 6
;; @002b                               v23 = uextend.i64 v6
;; @002b                               v27 = iadd v26, v23
;;                                     v187 = iconst.i64 0
;;                                     v188 = icmp sge v27, v187  ; v187 = 0
;; @002b                               brif v188, block4, block5(v27)
;;
;;                                 block4:
;; @002b                               store.i64 notrap aligned v27, v7
;; @002b                               v32 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v34 = load.i64 notrap aligned v7
;; @002b                               jump block5(v34)
;;
;;                                 block5(v131: i64):
;;                                     v143 = load.i32 notrap v177
;; @002b                               trapz v143, user16
;; @002b                               v36 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v35 = uextend.i64 v143
;; @002b                               v37 = iadd v36, v35
;; @002b                               v38 = iconst.i64 16
;; @002b                               v39 = iadd v37, v38  ; v38 = 16
;; @002b                               v40 = load.i32 user2 readonly region0 v39
;; @002b                               v42 = uextend.i64 v3
;; @002b                               v46 = iadd v42, v23
;; @002b                               v41 = uextend.i64 v40
;; @002b                               v47 = icmp ugt v46, v41
;; @002b                               trapnz v47, user17
;;                                     v140 = load.i32 notrap v176
;; @002b                               trapz v140, user16
;; @002b                               v57 = uextend.i64 v140
;; @002b                               v59 = iadd v36, v57
;; @002b                               v61 = iadd v59, v38  ; v38 = 16
;; @002b                               v62 = load.i32 user2 readonly region0 v61
;; @002b                               v64 = uextend.i64 v5
;; @002b                               v68 = iadd v64, v23
;; @002b                               v63 = uextend.i64 v62
;; @002b                               v69 = icmp ugt v68, v63
;; @002b                               trapnz v69, user17
;; @002b                               v85 = load.i64 notrap aligned v7+40
;; @002b                               v51 = iconst.i64 20
;; @002b                               v52 = iadd v37, v51  ; v51 = 20
;;                                     v179 = iconst.i64 2
;;                                     v180 = ishl v42, v179  ; v179 = 2
;; @002b                               v56 = iadd v52, v180
;;                                     v184 = ishl.i64 v23, v179  ; v179 = 2
;; @002b                               v87 = uadd_overflow_trap v56, v184, user2
;; @002b                               v86 = iadd v36, v85
;; @002b                               v88 = icmp ugt v87, v86
;; @002b                               trapnz v88, user2
;; @002b                               v74 = iadd v59, v51  ; v51 = 20
;;                                     v182 = ishl v64, v179  ; v179 = 2
;; @002b                               v78 = iadd v74, v182
;; @002b                               v92 = uadd_overflow_trap v78, v184, user2
;; @002b                               v93 = icmp ugt v92, v86
;; @002b                               trapnz v93, user2
;; @002b                               brif.i64 v23, block6, block9
;;
;;                                 block6:
;;                                     v134 = load.i32 notrap v177
;;                                     v135 = load.i32 notrap v176
;; @002b                               v94 = icmp.i64 ult v56, v78
;; @002b                               v99 = iadd.i64 v56, v184
;; @002b                               v100 = iadd.i64 v78, v184
;; @002b                               v102 = iadd.i32 v5, v6
;; @002b                               v54 = iconst.i64 4
;; @002b                               v125 = iconst.i32 1
;; @002b                               brif v94, block7(v56, v78, v5), block8(v99, v100, v102)
;;
;;                                 block7(v103: i64, v104: i64, v105: i32):
;; @002b                               v108 = load.i32 user2 little region0 v104
;; @002b                               store user2 little region0 v108, v103
;;                                     v194 = iconst.i64 4
;;                                     v195 = iadd v104, v194  ; v194 = 4
;; @002b                               v115 = icmp eq v195, v100
;;                                     v196 = iadd v103, v194  ; v194 = 4
;;                                     v197 = iconst.i32 1
;;                                     v198 = iadd v105, v197  ; v197 = 1
;; @002b                               brif v115, block9, block7(v196, v195, v198)
;;
;;                                 block8(v116: i64, v117: i64, v118: i32):
;;                                     v189 = iconst.i64 4
;;                                     v190 = isub v117, v189  ; v189 = 4
;; @002b                               v127 = load.i32 user2 little region0 v190
;;                                     v191 = isub v116, v189  ; v189 = 4
;; @002b                               store user2 little region0 v127, v191
;; @002b                               v128 = icmp eq v190, v78
;;                                     v192 = iconst.i32 1
;;                                     v193 = isub v118, v192  ; v192 = 1
;; @002b                               brif v128, block9, block8(v191, v190, v193)
;;
;;                                 block9:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v131, v7
;; @002f                               return
;; }
