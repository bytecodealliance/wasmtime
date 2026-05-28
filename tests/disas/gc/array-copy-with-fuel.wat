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
;;                                     v201 = stack_addr.i64 ss0
;;                                     store notrap v2, v201
;;                                     v200 = stack_addr.i64 ss1
;;                                     store notrap v4, v200
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;;                                     v198 = iconst.i64 1
;; @0020                               v9 = iadd v8, v198  ; v198 = 1
;; @0020                               v10 = iconst.i64 0
;; @0020                               v11 = icmp sge v9, v10  ; v10 = 0
;; @0020                               brif v11, block2, block3(v9)
;;
;;                                 block2:
;;                                     v210 = iadd.i64 v8, v198  ; v198 = 1
;; @0020                               store notrap aligned v210, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v16 = load.i64 notrap aligned v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v73: i64):
;;                                     v140 = load.i32 notrap v201
;; @002b                               trapz v140, user16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v22 = uextend.i64 v140
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
;;                                     v137 = load.i32 notrap v200
;; @002b                               trapz v137, user16
;; @002b                               v41 = uextend.i64 v137
;; @002b                               v43 = iadd v23, v41
;; @002b                               v45 = iadd v43, v25  ; v25 = 16
;; @002b                               v46 = load.i32 user2 readonly v45
;; @002b                               v48 = uextend.i64 v5
;; @002b                               v51 = iadd v48, v30
;; @002b                               v47 = uextend.i64 v46
;; @002b                               v52 = icmp ugt v51, v47
;; @002b                               trapnz v52, user17
;; @002b                               v64 = load.i64 notrap aligned v7+40
;;                                     v187 = iconst.i64 20
;; @002b                               v37 = iadd v24, v187  ; v187 = 20
;;                                     v203 = iconst.i64 2
;;                                     v204 = ishl v29, v203  ; v203 = 2
;; @002b                               v40 = iadd v37, v204
;;                                     v208 = ishl v30, v203  ; v203 = 2
;; @002b                               v66 = uadd_overflow_trap v40, v208, user2
;; @002b                               v65 = iadd v23, v64
;; @002b                               v67 = icmp ugt v66, v65
;; @002b                               trapnz v67, user2
;; @002b                               v56 = iadd v43, v187  ; v187 = 20
;;                                     v206 = ishl v48, v203  ; v203 = 2
;; @002b                               v59 = iadd v56, v206
;; @002b                               v71 = uadd_overflow_trap v59, v208, user2
;; @002b                               v72 = icmp ugt v71, v65
;; @002b                               trapnz v72, user2
;;                                     v165 = iconst.i64 6
;; @002b                               v74 = iadd v73, v165  ; v165 = 6
;; @002b                               brif v30, block4, block7(v74)
;;
;;                                 block4:
;;                                     v131 = load.i32 notrap v201
;;                                     v132 = load.i32 notrap v200
;; @002b                               v75 = icmp.i64 ult v40, v59
;;                                     v211 = iadd.i64 v73, v165  ; v165 = 6
;; @002b                               v78 = iadd.i64 v40, v208
;; @002b                               v79 = iadd.i64 v59, v208
;; @002b                               v81 = iadd.i32 v5, v6
;;                                     v186 = iconst.i64 4
;; @002b                               v119 = iconst.i32 1
;; @002b                               brif v75, block5(v40, v59, v5, v131, v132, v211), block6(v78, v79, v81, v131, v132, v211)
;;
;;                                 block5(v82: i64, v83: i64, v84: i32, v85: i32, v86: i32, v87: i64):
;;                                     store notrap v85, v201
;;                                     store notrap v86, v200
;;                                     v221 = iconst.i64 1
;;                                     v222 = iadd v87, v221  ; v221 = 1
;;                                     v223 = iconst.i64 0
;;                                     v224 = icmp sge v222, v223  ; v223 = 0
;; @002b                               brif v224, block8, block9(v222)
;;
;;                                 block6(v101: i64, v102: i64, v103: i32, v104: i32, v105: i32, v106: i64):
;;                                     store notrap v104, v200
;;                                     store notrap v105, v201
;;                                     v212 = iconst.i64 1
;;                                     v213 = iadd v106, v212  ; v212 = 1
;;                                     v214 = iconst.i64 0
;;                                     v215 = icmp sge v213, v214  ; v214 = 0
;; @002b                               brif v215, block10, block11(v213)
;;
;;                                 block7(v126: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v222, v7
;; @002b                               v93 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @002b                               v95 = load.i64 notrap aligned v7
;; @002b                               jump block9(v95)
;;
;;                                 block9(v123: i64):
;; @002b                               v96 = load.i32 user2 little v83
;; @002b                               store user2 little v96, v82
;;                                     v127 = load.i32 notrap v201
;;                                     v128 = load.i32 notrap v200
;;                                     v225 = iconst.i64 4
;;                                     v226 = iadd.i64 v83, v225  ; v225 = 4
;; @002b                               v100 = icmp eq v226, v79
;;                                     v227 = iadd.i64 v82, v225  ; v225 = 4
;;                                     v228 = iconst.i32 1
;;                                     v229 = iadd.i32 v84, v228  ; v228 = 1
;; @002b                               brif v100, block7(v123), block5(v227, v226, v229, v127, v128, v123)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v213, v7
;; @002b                               v112 = call fn0(v0), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
;; @002b                               v114 = load.i64 notrap aligned v7
;; @002b                               jump block11(v114)
;;
;;                                 block11(v124: i64):
;;                                     v216 = iconst.i64 4
;;                                     v217 = isub.i64 v102, v216  ; v216 = 4
;; @002b                               v121 = load.i32 user2 little v217
;;                                     v218 = isub.i64 v101, v216  ; v216 = 4
;; @002b                               store user2 little v121, v218
;;                                     v129 = load.i32 notrap v200
;;                                     v130 = load.i32 notrap v201
;; @002b                               v122 = icmp eq v217, v59
;;                                     v219 = iconst.i32 1
;;                                     v220 = isub.i32 v103, v219  ; v219 = 1
;; @002b                               brif v122, block7(v124), block6(v218, v217, v220, v129, v130, v124)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v126, v7
;; @002f                               return
;; }
