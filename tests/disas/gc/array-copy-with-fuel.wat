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
;;     fn0 = colocated u805306368:13 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;;                                     v145 = stack_addr.i64 ss0
;;                                     store notrap v2, v145
;;                                     v144 = stack_addr.i64 ss1
;;                                     store notrap v4, v144
;; @0020                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v8 = load.i64 notrap aligned v7
;;                                     v142 = iconst.i64 1
;; @0020                               v9 = iadd v8, v142  ; v142 = 1
;; @0020                               v10 = iconst.i64 0
;; @0020                               v11 = icmp sge v9, v10  ; v10 = 0
;; @0020                               brif v11, block2, block3(v9)
;;
;;                                 block2:
;;                                     v190 = iadd.i64 v8, v142  ; v142 = 1
;; @0020                               store notrap aligned v190, v7
;; @0020                               v14 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0020                               v16 = load.i64 notrap aligned v7
;; @0020                               jump block3(v16)
;;
;;                                 block3(v107: i64):
;;                                     v117 = load.i32 notrap v144
;; @002b                               trapz v117, user16
;; @002b                               v23 = load.i64 notrap aligned readonly can_move v7+32
;; @002b                               v22 = uextend.i64 v117
;; @002b                               v24 = iadd v23, v22
;; @002b                               v25 = iconst.i64 16
;; @002b                               v26 = iadd v24, v25  ; v25 = 16
;; @002b                               v27 = load.i32 user2 readonly v26
;; @002b                               v28 = uadd_overflow_trap.i32 v5, v6, user17
;; @002b                               v29 = icmp ugt v28, v27
;; @002b                               trapnz v29, user17
;; @002b                               v31 = uextend.i64 v27
;;                                     v146 = iconst.i64 2
;;                                     v147 = ishl v31, v146  ; v146 = 2
;;                                     v135 = iconst.i64 32
;; @002b                               v33 = ushr v147, v135  ; v135 = 32
;; @002b                               trapnz v33, user2
;;                                     v156 = iconst.i32 2
;;                                     v157 = ishl v27, v156  ; v156 = 2
;; @002b                               v35 = iconst.i32 20
;; @002b                               v36 = uadd_overflow_trap v157, v35, user2  ; v35 = 20
;; @002b                               v40 = uadd_overflow_trap v117, v36, user2
;;                                     v114 = load.i32 notrap v145
;; @002b                               trapz v114, user16
;; @002b                               v47 = uextend.i64 v114
;; @002b                               v49 = iadd v23, v47
;; @002b                               v51 = iadd v49, v25  ; v25 = 16
;; @002b                               v52 = load.i32 user2 readonly v51
;; @002b                               v53 = uadd_overflow_trap.i32 v3, v6, user17
;; @002b                               v54 = icmp ugt v53, v52
;; @002b                               trapnz v54, user17
;; @002b                               v56 = uextend.i64 v52
;;                                     v167 = ishl v56, v146  ; v146 = 2
;; @002b                               v58 = ushr v167, v135  ; v135 = 32
;; @002b                               trapnz v58, user2
;;                                     v174 = ishl v52, v156  ; v156 = 2
;; @002b                               v61 = uadd_overflow_trap v174, v35, user2  ; v35 = 20
;; @002b                               v65 = uadd_overflow_trap v114, v61, user2
;; @002b                               brif.i32 v6, block4, block7(v107)
;;
;;                                 block4:
;; @002b                               v66 = uextend.i64 v65
;; @002b                               v68 = iadd.i64 v23, v66
;;                                     v191 = iconst.i32 2
;;                                     v192 = ishl.i32 v3, v191  ; v191 = 2
;;                                     v193 = iconst.i32 20
;;                                     v194 = iadd v192, v193  ; v193 = 20
;; @002b                               v69 = isub.i32 v61, v194
;; @002b                               v70 = uextend.i64 v69
;; @002b                               v71 = isub v68, v70
;; @002b                               v41 = uextend.i64 v40
;; @002b                               v43 = iadd.i64 v23, v41
;;                                     v195 = ishl.i32 v5, v191  ; v191 = 2
;;                                     v196 = iadd v195, v193  ; v193 = 20
;; @002b                               v44 = isub.i32 v36, v196
;; @002b                               v45 = uextend.i64 v44
;; @002b                               v46 = isub v43, v45
;; @002b                               v77 = icmp ult v71, v46
;;                                     v197 = ishl.i32 v6, v191  ; v191 = 2
;; @002b                               v73 = uextend.i64 v197
;; @002b                               v75 = iadd v71, v73
;; @002b                               v74 = iadd v46, v73
;; @002b                               v30 = iconst.i64 4
;;                                     v187 = iadd v43, v30  ; v30 = 4
;;                                     v123 = iconst.i64 6
;; @002b                               brif v77, block5(v71, v46, v107), block6(v75, v74, v107)
;;
;;                                 block5(v78: i64, v79: i64, v80: i64):
;;                                     v205 = iconst.i64 6
;;                                     v206 = iadd v80, v205  ; v205 = 6
;;                                     v207 = iconst.i64 0
;;                                     v208 = icmp sge v206, v207  ; v207 = 0
;; @002b                               brif v208, block8, block9(v206)
;;
;;                                 block6(v93: i64, v94: i64, v96: i64):
;;                                     v198 = iconst.i64 0
;;                                     v199 = icmp sge v96, v198  ; v198 = 0
;; @002b                               brif v199, block10, block11(v96)
;;
;;                                 block7(v111: i64):
;; @002f                               jump block1
;;
;;                                 block8:
;; @002b                               store.i64 notrap aligned v206, v7
;; @002b                               v86 = call fn0(v0)
;; @002b                               v88 = load.i64 notrap aligned v7
;; @002b                               jump block9(v88)
;;
;;                                 block9(v108: i64):
;; @002b                               v89 = load.i32 user2 little v79
;; @002b                               store user2 little v89, v78
;;                                     v209 = iconst.i64 4
;;                                     v210 = iadd.i64 v79, v209  ; v209 = 4
;; @002b                               v92 = icmp eq v210, v74
;;                                     v211 = iadd.i64 v78, v209  ; v209 = 4
;; @002b                               brif v92, block7(v108), block5(v211, v210, v108)
;;
;;                                 block10:
;; @002b                               store.i64 notrap aligned v96, v7
;; @002b                               v100 = call fn0(v0)
;; @002b                               v102 = load.i64 notrap aligned v7
;; @002b                               jump block11(v102)
;;
;;                                 block11(v109: i64):
;;                                     v200 = iconst.i64 4
;;                                     v201 = isub.i64 v94, v200  ; v200 = 4
;; @002b                               v105 = load.i32 user2 little v201
;;                                     v202 = isub.i64 v93, v200  ; v200 = 4
;; @002b                               store user2 little v105, v202
;;                                     v186 = iadd.i64 v94, v45
;;                                     v203 = iadd.i64 v43, v30  ; v30 = 4
;;                                     v204 = icmp eq v186, v203
;; @002b                               brif v204, block7(v109), block6(v202, v201, v109)
;;
;;                                 block1:
;; @002f                               store.i64 notrap aligned v111, v7
;; @002f                               return
;; }
