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
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:12 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;;                                     v179 = stack_addr.i64 ss0
;;                                     store notrap v2, v179
;;                                     v180 = stack_addr.i64 ss1
;;                                     store notrap v4, v180
;; @0020                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;; @0020                               v9 = iconst.i64 1
;; @0020                               v10 = iadd v8, v9  ; v9 = 1
;; @0020                               v11 = iconst.i64 0
;; @0020                               v12 = icmp sge v10, v11  ; v11 = 0
;; @0020                               brif v12, block2, block3(v10)
;;
;;                                 block2:
;;                                     v193 = iadd.i64 v8, v9  ; v9 = 1
;; @0020                               store notrap aligned v193, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v16 = load.i64 notrap aligned v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v87: i64):
;;                                     v178 = load.i32 notrap v179
;; @002b                               trapz v178, user16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v22 = uextend.i64 v178
;; @002b                               v24 = iadd v23, v22
;; @002b                               v25 = iconst.i64 16
;; @002b                               v26 = iadd v24, v25  ; v25 = 16
;; @002b                               v27 = load.i32 user2 readonly region1 v26
;; @002b                               v29 = uextend.i64 v3
;; @002b                               v30 = uextend.i64 v6
;; @002b                               v33 = iadd v29, v30
;; @002b                               v28 = uextend.i64 v27
;; @002b                               v34 = icmp ugt v33, v28
;; @002b                               trapnz v34, user17
;;                                     v172 = load.i32 notrap v180
;; @002b                               trapz v172, user16
;; @002b                               v45 = uextend.i64 v172
;; @002b                               v47 = iadd v23, v45
;; @002b                               v49 = iadd v47, v25  ; v25 = 16
;; @002b                               v50 = load.i32 user2 readonly region1 v49
;; @002b                               v52 = uextend.i64 v5
;; @002b                               v56 = iadd v52, v30
;; @002b                               v51 = uextend.i64 v50
;; @002b                               v57 = icmp ugt v56, v51
;; @002b                               trapnz v57, user17
;; @002b                               v76 = load.i64 notrap aligned v7+40
;; @002b                               v39 = iconst.i64 20
;; @002b                               v40 = iadd v24, v39  ; v39 = 20
;;                                     v186 = iconst.i64 2
;;                                     v187 = ishl v29, v186  ; v186 = 2
;; @002b                               v44 = iadd v40, v187
;;                                     v191 = ishl v30, v186  ; v186 = 2
;; @002b                               v78 = uadd_overflow_trap v44, v191, user2
;; @002b                               v77 = iadd v23, v76
;; @002b                               v79 = icmp ugt v78, v77
;; @002b                               trapnz v79, user2
;; @002b                               v63 = iadd v47, v39  ; v39 = 20
;;                                     v189 = ishl v52, v186  ; v186 = 2
;; @002b                               v67 = iadd v63, v189
;; @002b                               v85 = uadd_overflow_trap v67, v191, user2
;; @002b                               v86 = icmp ugt v85, v77
;; @002b                               trapnz v86, user2
;; @002b                               v88 = iconst.i64 6
;; @002b                               v89 = iadd v87, v88  ; v88 = 6
;; @002b                               brif.i32 v6, block4, block7(v89)
;;
;;                                 block4:
;;                                     v160 = load.i32 notrap v179
;;                                     v162 = load.i32 notrap v180
;; @002b                               v90 = icmp.i64 ult v44, v67
;;                                     v194 = iadd.i64 v87, v88  ; v88 = 6
;; @002b                               v95 = iadd.i64 v44, v191
;; @002b                               v96 = iadd.i64 v67, v191
;; @002b                               v98 = iadd.i32 v5, v6
;; @002b                               v42 = iconst.i64 4
;; @002b                               v139 = iconst.i32 1
;; @002b                               brif v90, block5(v44, v67, v5, v160, v162, v194), block6(v95, v96, v98, v160, v162, v194)
;;
;;                                 block5(v99: i64, v100: i64, v101: i32, v102: i32, v103: i32, v104: i64):
;;                                     store notrap v102, v179
;;                                     store notrap v103, v180
;;                                     v204 = iconst.i64 1
;;                                     v205 = iadd v104, v204  ; v204 = 1
;;                                     v206 = iconst.i64 0
;;                                     v207 = icmp sge v205, v206  ; v206 = 0
;; @002b                               brif v207, block8, block9(v205)
;;
;;                                 block6(v121: i64, v122: i64, v123: i32, v124: i32, v125: i32, v126: i64):
;;                                     store notrap v124, v180
;;                                     store notrap v125, v179
;;                                     v195 = iconst.i64 1
;;                                     v196 = iadd v126, v195  ; v195 = 1
;;                                     v197 = iconst.i64 0
;;                                     v198 = icmp sge v196, v197  ; v197 = 0
;; @002b                               brif v198, block10, block11(v196)
;;
;;                                 block7(v146: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v205, v7
;; @002b                               v110 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v112 = load.i64 notrap aligned v7
;; @002b                               jump block9(v112)
;;
;;                                 block9(v143: i64):
;; @002b                               v113 = load.i32 user2 little region1 v100
;; @002b                               store user2 little region1 v113, v99
;;                                     v148 = load.i32 notrap v179
;;                                     v150 = load.i32 notrap v180
;;                                     v208 = iconst.i64 4
;;                                     v209 = iadd.i64 v100, v208  ; v208 = 4
;; @002b                               v120 = icmp eq v209, v96
;;                                     v210 = iadd.i64 v99, v208  ; v208 = 4
;;                                     v211 = iconst.i32 1
;;                                     v212 = iadd.i32 v101, v211  ; v211 = 1
;; @002b                               brif v120, block7(v143), block5(v210, v209, v212, v148, v150, v143)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v196, v7
;; @002b                               v132 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @002b                               v134 = load.i64 notrap aligned v7
;; @002b                               jump block11(v134)
;;
;;                                 block11(v144: i64):
;;                                     v199 = iconst.i64 4
;;                                     v200 = isub.i64 v122, v199  ; v199 = 4
;; @002b                               v141 = load.i32 user2 little region1 v200
;;                                     v201 = isub.i64 v121, v199  ; v199 = 4
;; @002b                               store user2 little region1 v141, v201
;;                                     v154 = load.i32 notrap v180
;;                                     v156 = load.i32 notrap v179
;; @002b                               v142 = icmp eq v200, v67
;;                                     v202 = iconst.i32 1
;;                                     v203 = isub.i32 v123, v202  ; v202 = 1
;; @002b                               brif v142, block7(v144), block6(v201, v200, v203, v154, v156, v144)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v146, v7
;; @002f                               return
;; }
