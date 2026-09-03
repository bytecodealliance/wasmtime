;;! target = "x86_64-unknown-linux-gnu"
;;! flags = "-W stack-switching=y -W exceptions=y -W function-references=y"

(module
  (type $fta (func))
  (type $cta (cont $fta))

  (type $ftb (func (param (ref $cta))))
  (type $ctb (cont $ftb))

  (tag $yield)

  (func $task_a (type $fta)
    (cont.new $ctb (ref.func $task_b))
    (switch $ctb $yield)
  )

  (func $task_b (type $ftb))

  (elem declare func $task_a $task_b)

  (func (export "entry")
    (cont.new $cta (ref.func $task_a))
    (resume $cta (on $yield switch))
  )
)

;; function u0:0(i64 vmctx, i64) tail {
;;     ss0 = explicit_slot 16, align = 65536
;;     ss1 = explicit_slot 24, align = 256
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1073741824 "VMContRef+0x0"
;;     region3 = 67108952 "VMStoreContext+0x58"
;;     region4 = 1140850688 "ContinuationStackMemory+0x0"
;;     region5 = 67108936 "VMStoreContext+0x48"
;;     region6 = 67108928 "VMStoreContext+0x40"
;;     region7 = 67108944 "VMStoreContext+0x50"
;;     region8 = 1543503873 "Stack(ss1)"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32, i32) -> i64 tail
;;     sig2 = (i64 vmctx, i32) -> i8 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     fn2 = colocated u805306368:44 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003a                               v2 = iconst.i32 1
;; @003a                               v3 = call fn0(v0, v2)  ; v2 = 1
;; @003c                               trapz v3, user16
;; @003c                               v4 = iconst.i32 1
;; @003c                               v5 = iconst.i32 0
;; @003c                               v6 = iconst.i32 0
;; @003c                               v7 = call fn1(v0, v3, v4, v5, v6)  ; v4 = 1, v5 = 0, v6 = 0
;; @003c                               v8 = load.i64 notrap aligned region2 v7+88
;; @003c                               v9 = uextend.i128 v7
;; @003c                               v10 = uextend.i128 v8
;; @003c                               v11 = iconst.i64 64
;; @003c                               v12 = uextend.i128 v11  ; v11 = 64
;; @003c                               v13 = ishl v10, v12
;; @003c                               v14 = bor v13, v9
;; @003e                               v15 = ireduce.i64 v14
;; @003e                               v16 = iconst.i64 64
;; @003e                               v17 = uextend.i128 v16  ; v16 = 64
;; @003e                               v18 = ushr v14, v17
;; @003e                               v19 = ireduce.i64 v18
;; @003e                               trapz v15, user16
;; @003e                               v20 = load.i64 notrap aligned region2 v15+88
;; @003e                               v21 = icmp eq v20, v19
;; @003e                               trapz v21, user23
;; @003e                               v22 = iconst.i64 1
;; @003e                               v23 = iadd v20, v22  ; v22 = 1
;; @003e                               store notrap aligned region2 v23, v15+88
;; @003e                               v24 = iconst.i64 48
;; @003e                               v25 = iadd v0, v24  ; v24 = 48
;; @003e                               v26 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v27 = load.i64 notrap aligned region3 v26+88
;; @003e                               v28 = load.i64 notrap aligned region3 v26+96
;; @003e                               jump block2(v27, v28)
;;
;;                                 block2(v29: i64, v30: i64):
;; @003e                               v31 = iconst.i64 1
;; @003e                               v32 = icmp eq v29, v31  ; v31 = 1
;; @003e                               brif v32, block7, block3
;;
;;                                 block3:
;; @003e                               v33 = load.i64 notrap aligned region2 v30+64
;; @003e                               v34 = load.i64 notrap aligned region2 v30+72
;; @003e                               v35 = iconst.i64 40
;; @003e                               v36 = iadd v34, v35  ; v35 = 40
;; @003e                               v37 = load.i64 notrap aligned region2 v36+8
;; @003e                               v38 = load.i32 notrap aligned region2 v34+56
;; @003e                               v39 = load.i32 notrap aligned region2 v36
;; @003e                               jump block4(v38)
;;
;;                                 block4(v40: i32):
;; @003e                               v41 = icmp ult v40, v39
;; @003e                               brif v41, block5, block2(v33, v34)
;;
;;                                 block5:
;; @003e                               v42 = iconst.i32 8
;; @003e                               v43 = imul.i32 v40, v42  ; v42 = 8
;; @003e                               v44 = uextend.i64 v43
;; @003e                               v45 = iadd.i64 v37, v44
;; @003e                               v46 = load.i64 notrap aligned region4 v45
;; @003e                               v47 = icmp eq v46, v25
;; @003e                               v48 = iconst.i32 1
;; @003e                               v49 = iadd.i32 v40, v48  ; v48 = 1
;; @003e                               brif v47, block6, block4(v49)
;;
;;                                 block7 cold:
;; @003e                               trap user22
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned region2 v30, v28+80
;; @003e                               v50 = iconst.i64 144
;; @003e                               v51 = iadd.i64 v28, v50  ; v50 = 144
;; @003e                               v52 = iconst.i32 1
;; @003e                               v53 = stack_addr.i64 ss0
;; @003e                               store notrap aligned region2 v52, v51+4  ; v52 = 1
;; @003e                               store notrap aligned region2 v53, v51+8
;; @003e                               v54 = iconst.i64 0
;; @003e                               v55 = iadd.i64 v28, v54  ; v54 = 0
;; @003e                               v56 = iconst.i32 3
;; @003e                               v57 = iconst.i64 32
;; @003e                               v58 = iadd v55, v57  ; v57 = 32
;; @003e                               store notrap aligned region2 v56, v58  ; v56 = 3
;; @003e                               v59 = iconst.i64 0
;; @003e                               v60 = iconst.i64 0
;; @003e                               store notrap aligned region2 v59, v30+64  ; v59 = 0
;; @003e                               store notrap aligned region2 v60, v30+72  ; v60 = 0
;; @003e                               v61 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v62 = iconst.i64 0
;; @003e                               v63 = iadd v55, v62  ; v62 = 0
;; @003e                               v64 = load.i64 notrap aligned region5 v61+72
;; @003e                               v65 = load.i64 notrap aligned region6 v61+64
;; @003e                               v66 = load.i64 notrap aligned region7 v61+80
;; @003e                               store notrap aligned region2 v64, v63+8
;; @003e                               store notrap aligned region2 v65, v63+16
;; @003e                               store notrap aligned region2 v66, v63+24
;; @003e                               v67 = load.i64 notrap aligned region2 v28+88
;; @003e                               v68 = uextend.i128 v28
;; @003e                               v69 = uextend.i128 v67
;; @003e                               v70 = iconst.i64 64
;; @003e                               v71 = uextend.i128 v70  ; v70 = 64
;; @003e                               v72 = ishl v69, v71
;; @003e                               v73 = bor v72, v68
;; @003e                               v75 = iconst.i64 0
;; @003e                               v76 = iadd.i64 v15, v75  ; v75 = 0
;; @003e                               v77 = iconst.i64 32
;; @003e                               v78 = iadd v76, v77  ; v77 = 32
;; @003e                               v79 = load.i32 notrap aligned region2 v78
;; @003e                               v80 = iconst.i32 0
;; @003e                               v81 = icmp ne v79, v80  ; v80 = 0
;; @003e                               brif v81, block9, block8
;;
;;                                 block8:
;; @003e                               v82 = iconst.i64 120
;; @003e                               v83 = iadd.i64 v15, v82  ; v82 = 120
;; @003e                               v84 = load.i64 notrap aligned region2 v83+8
;; @003e                               v85 = load.i32 notrap aligned region2 v83
;; @003e                               v86 = iconst.i32 1
;; @003e                               v87 = iadd v85, v86  ; v86 = 1
;; @003e                               store notrap aligned region2 v87, v83
;; @003e                               v88 = uextend.i64 v85
;; @003e                               v89 = iconst.i64 16
;; @003e                               v90 = imul v88, v89  ; v89 = 16
;; @003e                               v91 = iadd v84, v90
;; @003e                               jump block10(v91)
;;
;;                                 block9:
;; @003e                               v92 = iconst.i64 144
;; @003e                               v93 = iadd.i64 v15, v92  ; v92 = 144
;; @003e                               v94 = load.i64 notrap aligned region2 v93+8
;; @003e                               v95 = load.i32 notrap aligned region2 v93
;; @003e                               v96 = iconst.i32 1
;; @003e                               v97 = iadd v95, v96  ; v96 = 1
;; @003e                               store notrap aligned region2 v97, v93
;; @003e                               v98 = uextend.i64 v95
;; @003e                               v99 = iconst.i64 16
;; @003e                               v100 = imul v98, v99  ; v99 = 16
;; @003e                               v101 = iadd v94, v100
;; @003e                               jump block10(v101)
;;
;;                                 block10(v74: i64):
;; @003e                               store.i128 notrap aligned region4 v73, v74
;; @003e                               v102 = iconst.i64 0
;; @003e                               v103 = iadd.i64 v15, v102  ; v102 = 0
;; @003e                               v104 = iconst.i32 1
;; @003e                               v105 = iconst.i64 32
;; @003e                               v106 = iadd v103, v105  ; v105 = 32
;; @003e                               store notrap aligned region2 v104, v106  ; v104 = 1
;; @003e                               v107 = load.i64 notrap aligned region2 v15+80
;; @003e                               store.i64 notrap aligned region2 v33, v107+64
;; @003e                               store.i64 notrap aligned region2 v34, v107+72
;; @003e                               v108 = iconst.i64 2
;; @003e                               v109 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               store notrap aligned region3 v108, v109+88  ; v108 = 2
;; @003e                               store.i64 notrap aligned region3 v15, v109+96
;; @003e                               v110 = iconst.i64 0
;; @003e                               v111 = iadd v103, v110  ; v110 = 0
;; @003e                               v112 = load.i64 notrap aligned region2 v111
;; @003e                               store notrap aligned region1 v112, v61+24
;; @003e                               v113 = load.i64 notrap aligned region2 v111+8
;; @003e                               store notrap aligned region5 v113, v61+72
;; @003e                               v114 = load.i64 notrap aligned region2 v111+16
;; @003e                               store notrap aligned region6 v114, v61+64
;; @003e                               v115 = load.i64 notrap aligned region2 v111+24
;; @003e                               store notrap aligned region7 v115, v61+80
;; @003e                               v116 = iconst.i64 96
;; @003e                               v117 = iadd.i64 v30, v116  ; v116 = 96
;; @003e                               v118 = load.i64 notrap aligned region2 v117
;; @003e                               v119 = iconst.i64 -24
;; @003e                               v120 = iadd v118, v119  ; v119 = -24
;; @003e                               v121 = iconst.i64 96
;; @003e                               v122 = iadd v107, v121  ; v121 = 96
;; @003e                               v123 = load.i64 notrap aligned region2 v122
;; @003e                               v124 = iconst.i64 -24
;; @003e                               v125 = iadd v123, v124  ; v124 = -24
;; @003e                               v126 = stack_addr.i64 ss1
;; @003e                               v127 = load.i64 notrap aligned region4 v125
;; @003e                               store notrap aligned region8 v127, v126
;; @003e                               v128 = load.i64 notrap aligned region4 v120
;; @003e                               store notrap aligned region4 v128, v125
;; @003e                               v129 = load.i64 notrap aligned region4 v125+8
;; @003e                               store notrap aligned region8 v129, v126+8
;; @003e                               v130 = load.i64 notrap aligned region4 v120+8
;; @003e                               store notrap aligned region4 v130, v125+8
;; @003e                               v131 = load.i64 notrap aligned region4 v125+16
;; @003e                               store notrap aligned region8 v131, v126+16
;; @003e                               v132 = load.i64 notrap aligned region4 v120+16
;; @003e                               store notrap aligned region4 v132, v125+16
;; @003e                               v133 = iconst.i64 3
;; @003e                               v134 = iconst.i64 32
;; @003e                               v135 = ishl v133, v134  ; v133 = 3, v134 = 32
;; @003e                               v136 = stack_switch v120, v126, v135
;; @003e                               v137 = iconst.i64 32
;; @003e                               v138 = ushr v136, v137  ; v137 = 32
;; @003e                               v139 = iconst.i64 5
;; @003e                               v140 = icmp eq v138, v139  ; v139 = 5
;; @003e                               brif v140, block11, block12
;;
;;                                 block11 cold:
;; @003e                               v141 = iconst.i64 144
;; @003e                               v142 = iadd.i64 v28, v141  ; v141 = 144
;; @003e                               v143 = load.i64 notrap aligned region2 v142+8
;; @003e                               v144 = load.i32 notrap aligned region4 v143
;; @003e                               v145 = iconst.i32 0
;; @003e                               store notrap aligned region2 v145, v142  ; v145 = 0
;; @003e                               v146 = iconst.i32 0
;; @003e                               store notrap aligned region2 v146, v142+4  ; v146 = 0
;; @003e                               v147 = iconst.i64 0
;; @003e                               store notrap aligned region2 v147, v142+8  ; v147 = 0
;; @003e                               try_call fn2(v0, v144), sig2, block13, [ context v0 ]
;;
;;                                 block13:
;; @003e                               trap user12
;;
;;                                 block12:
;; @003e                               v148 = iconst.i64 144
;; @003e                               v149 = iadd.i64 v28, v148  ; v148 = 144
;; @003e                               v150 = load.i64 notrap aligned region2 v149+8
;; @003e                               v151 = iconst.i32 0
;; @003e                               store notrap aligned region2 v151, v149  ; v151 = 0
;; @003e                               v152 = iconst.i32 0
;; @003e                               store notrap aligned region2 v152, v149+4  ; v152 = 0
;; @003e                               v153 = iconst.i64 0
;; @003e                               store notrap aligned region2 v153, v149+8  ; v153 = 0
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i128) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i128):
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return
;; }
;;
;; function u0:2(i64 vmctx, i64) tail {
;;     ss0 = explicit_slot 8, align = 256
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1073741824 "VMContRef+0x0"
;;     region3 = 67108952 "VMStoreContext+0x58"
;;     region4 = 67108936 "VMStoreContext+0x48"
;;     region5 = 67108928 "VMStoreContext+0x40"
;;     region6 = 67108944 "VMStoreContext+0x50"
;;     region7 = 1140850688 "ContinuationStackMemory+0x0"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32, i32) -> i64 tail
;;     sig2 = (i64 vmctx) tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     fn2 = colocated u805306368:41 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0047                               v2 = iconst.i32 0
;; @0047                               v3 = call fn0(v0, v2)  ; v2 = 0
;; @0049                               trapz v3, user16
;; @0049                               v4 = iconst.i32 0
;; @0049                               v5 = iconst.i32 0
;; @0049                               v6 = iconst.i32 0
;; @0049                               v7 = call fn1(v0, v3, v4, v5, v6)  ; v4 = 0, v5 = 0, v6 = 0
;; @0049                               v8 = load.i64 notrap aligned region2 v7+88
;; @0049                               v9 = uextend.i128 v7
;; @0049                               v10 = uextend.i128 v8
;; @0049                               v11 = iconst.i64 64
;; @0049                               v12 = uextend.i128 v11  ; v11 = 64
;; @0049                               v13 = ishl v10, v12
;; @0049                               v14 = bor v13, v9
;; @004b                               jump block2
;;
;;                                 block2:
;; @004b                               v15 = ireduce.i64 v14
;; @004b                               v16 = iconst.i64 64
;; @004b                               v17 = uextend.i128 v16  ; v16 = 64
;; @004b                               v18 = ushr.i128 v14, v17
;; @004b                               v19 = ireduce.i64 v18
;; @004b                               trapz v15, user16
;; @004b                               v20 = load.i64 notrap aligned region2 v15+88
;; @004b                               v21 = icmp eq v20, v19
;; @004b                               trapz v21, user23
;; @004b                               v22 = iconst.i64 1
;; @004b                               v23 = iadd v20, v22  ; v22 = 1
;; @004b                               store notrap aligned region2 v23, v15+88
;; @004b                               v24 = load.i64 notrap aligned region2 v15+80
;; @004b                               v25 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v26 = load.i64 notrap aligned region3 v25+88
;; @004b                               v27 = load.i64 notrap aligned region3 v25+96
;; @004b                               store notrap aligned region2 v26, v24+64
;; @004b                               store notrap aligned region2 v27, v24+72
;; @004b                               v28 = iconst.i64 0
;; @004b                               store notrap aligned region2 v28, v15+80  ; v28 = 0
;; @004b                               v29 = iconst.i64 2
;; @004b                               v30 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region3 v29, v30+88  ; v29 = 2
;; @004b                               store notrap aligned region3 v15, v30+96
;; @004b                               v31 = iconst.i64 0
;; @004b                               v32 = iadd v15, v31  ; v31 = 0
;; @004b                               v33 = iconst.i32 1
;; @004b                               v34 = iconst.i64 32
;; @004b                               v35 = iadd v32, v34  ; v34 = 32
;; @004b                               store notrap aligned region2 v33, v35  ; v33 = 1
;; @004b                               v36 = iconst.i32 2
;; @004b                               v37 = iconst.i64 32
;; @004b                               v38 = iadd v27, v37  ; v37 = 32
;; @004b                               store notrap aligned region2 v36, v38  ; v36 = 2
;; @004b                               v39 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v40 = iconst.i64 0
;; @004b                               v41 = iadd v27, v40  ; v40 = 0
;; @004b                               v42 = load.i64 notrap aligned region4 v39+72
;; @004b                               v43 = load.i64 notrap aligned region5 v39+64
;; @004b                               v44 = load.i64 notrap aligned region6 v39+80
;; @004b                               store notrap aligned region2 v42, v41+8
;; @004b                               store notrap aligned region2 v43, v41+16
;; @004b                               store notrap aligned region2 v44, v41+24
;; @004b                               v45 = load.i64 notrap aligned region1 v39+24
;; @004b                               store notrap aligned region2 v45, v41
;; @004b                               v46 = iconst.i64 0
;; @004b                               v47 = iadd v32, v46  ; v46 = 0
;; @004b                               v48 = load.i64 notrap aligned region2 v47
;; @004b                               store notrap aligned region1 v48, v39+24
;; @004b                               v49 = load.i64 notrap aligned region2 v47+8
;; @004b                               store notrap aligned region4 v49, v39+72
;; @004b                               v50 = load.i64 notrap aligned region2 v47+16
;; @004b                               store notrap aligned region5 v50, v39+64
;; @004b                               v51 = load.i64 notrap aligned region2 v47+24
;; @004b                               store notrap aligned region6 v51, v39+80
;; @004b                               v52 = iconst.i64 40
;; @004b                               v53 = iadd v27, v52  ; v52 = 40
;; @004b                               v54 = iconst.i32 1
;; @004b                               v55 = stack_addr.i64 ss0
;; @004b                               store notrap aligned region2 v54, v53+4  ; v54 = 1
;; @004b                               store notrap aligned region2 v55, v53+8
;; @004b                               v56 = iconst.i64 48
;; @004b                               v57 = iadd.i64 v0, v56  ; v56 = 48
;; @004b                               v58 = iconst.i32 1
;; @004b                               v59 = load.i64 notrap aligned region2 v53+8
;; @004b                               store notrap aligned region7 v57, v59
;; @004b                               store notrap aligned region2 v58, v53  ; v58 = 1
;; @004b                               v60 = iconst.i32 0
;; @004b                               store notrap aligned region2 v60, v27+56  ; v60 = 0
;; @004b                               v61 = iconst.i64 1
;; @004b                               v62 = iconst.i64 32
;; @004b                               v63 = ishl v61, v62  ; v61 = 1, v62 = 32
;; @004b                               v64 = iconst.i64 96
;; @004b                               v65 = iadd v24, v64  ; v64 = 96
;; @004b                               v66 = load.i64 notrap aligned region2 v65
;; @004b                               v67 = iconst.i64 -24
;; @004b                               v68 = iadd v66, v67  ; v67 = -24
;; @004b                               v69 = stack_switch v68, v68, v63
;; @004b                               v70 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v71 = load.i64 notrap aligned region3 v70+88
;; @004b                               v72 = load.i64 notrap aligned region3 v70+96
;; @004b                               v73 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region3 v26, v73+88
;; @004b                               store notrap aligned region3 v27, v73+96
;; @004b                               v74 = iconst.i32 1
;; @004b                               v75 = iconst.i64 32
;; @004b                               v76 = iadd v27, v75  ; v75 = 32
;; @004b                               store notrap aligned region2 v74, v76  ; v74 = 1
;; @004b                               v77 = iconst.i32 0
;; @004b                               store notrap aligned region2 v77, v53  ; v77 = 0
;; @004b                               v78 = iconst.i32 0
;; @004b                               store notrap aligned region2 v78, v53+4  ; v78 = 0
;; @004b                               v79 = iconst.i64 0
;; @004b                               store notrap aligned region2 v79, v53+8  ; v79 = 0
;; @004b                               store notrap aligned region2 v28, v27+56  ; v28 = 0
;; @004b                               brif v69, block6, block3
;;
;;                                 block6:
;; @004b                               v80 = iconst.i64 32
;; @004b                               v81 = ushr.i64 v69, v80  ; v80 = 32
;; @004b                               v82 = iconst.i64 4
;; @004b                               v83 = icmp eq v81, v82  ; v82 = 4
;; @004b                               brif v83, block5, block4
;;
;;                                 block5 cold:
;; @004b                               v84 = iconst.i64 0
;; @004b                               v85 = iadd.i64 v72, v84  ; v84 = 0
;; @004b                               v86 = iconst.i32 5
;; @004b                               v87 = iconst.i64 32
;; @004b                               v88 = iadd v85, v87  ; v87 = 32
;; @004b                               store notrap aligned region2 v86, v88  ; v86 = 5
;; @004b                               v89 = iconst.i64 0
;; @004b                               v90 = iadd.i64 v27, v89  ; v89 = 0
;; @004b                               v91 = load.i64 notrap aligned region2 v90
;; @004b                               store notrap aligned region1 v91, v39+24
;; @004b                               v92 = load.i64 notrap aligned region2 v90+8
;; @004b                               store notrap aligned region4 v92, v39+72
;; @004b                               v93 = load.i64 notrap aligned region2 v90+16
;; @004b                               store notrap aligned region5 v93, v39+64
;; @004b                               v94 = load.i64 notrap aligned region2 v90+24
;; @004b                               store notrap aligned region6 v94, v39+80
;; @004b                               v95 = iconst.i64 120
;; @004b                               v96 = iadd.i64 v72, v95  ; v95 = 120
;; @004b                               v97 = iconst.i32 0
;; @004b                               store notrap aligned region2 v97, v96  ; v97 = 0
;; @004b                               v98 = iconst.i32 0
;; @004b                               store notrap aligned region2 v98, v96+4  ; v98 = 0
;; @004b                               v99 = iconst.i64 0
;; @004b                               store notrap aligned region2 v99, v96+8  ; v99 = 0
;; @004b                               v100 = iconst.i64 144
;; @004b                               v101 = iadd.i64 v72, v100  ; v100 = 144
;; @004b                               v102 = iconst.i32 0
;; @004b                               store notrap aligned region2 v102, v101  ; v102 = 0
;; @004b                               v103 = iconst.i32 0
;; @004b                               store notrap aligned region2 v103, v101+4  ; v103 = 0
;; @004b                               v104 = iconst.i64 0
;; @004b                               store notrap aligned region2 v104, v101+8  ; v104 = 0
;; @004b                               try_call fn2(v0), sig2, block8, [ context v0 ]
;;
;;                                 block8:
;; @004b                               trap user12
;;
;;                                 block4:
;; @004b                               v105 = iconst.i64 0
;; @004b                               v106 = iadd.i64 v72, v105  ; v105 = 0
;; @004b                               v107 = iconst.i64 0
;; @004b                               v108 = iadd v106, v107  ; v107 = 0
;; @004b                               v109 = load.i64 notrap aligned region4 v39+72
;; @004b                               v110 = load.i64 notrap aligned region5 v39+64
;; @004b                               v111 = load.i64 notrap aligned region6 v39+80
;; @004b                               store notrap aligned region2 v109, v108+8
;; @004b                               store notrap aligned region2 v110, v108+16
;; @004b                               store notrap aligned region2 v111, v108+24
;; @004b                               v112 = iconst.i64 0
;; @004b                               v113 = iadd.i64 v27, v112  ; v112 = 0
;; @004b                               v114 = load.i64 notrap aligned region2 v113
;; @004b                               store notrap aligned region1 v114, v39+24
;; @004b                               v115 = load.i64 notrap aligned region2 v113+8
;; @004b                               store notrap aligned region4 v115, v39+72
;; @004b                               v116 = load.i64 notrap aligned region2 v113+16
;; @004b                               store notrap aligned region5 v116, v39+64
;; @004b                               v117 = load.i64 notrap aligned region2 v113+24
;; @004b                               store notrap aligned region6 v117, v39+80
;; @004b                               v118 = ireduce.i32 v69
;; @004b                               v119 = load.i64 notrap aligned region2 v72+88
;; @004b                               v120 = uextend.i128 v72
;; @004b                               v121 = uextend.i128 v119
;; @004b                               v122 = iconst.i64 64
;; @004b                               v123 = uextend.i128 v122  ; v122 = 64
;; @004b                               v124 = ishl v121, v123
;; @004b                               v125 = bor v124, v120
;; @004b                               jump block7
;;
;;                                 block9 cold:
;; @004b                               trap user12
;;
;;                                 block7:
;; @004b                               br_table v118, block9, []
;;
;;                                 block3:
;; @004b                               v126 = iconst.i64 0
;; @004b                               v127 = iadd.i64 v27, v126  ; v126 = 0
;; @004b                               v128 = load.i64 notrap aligned region2 v127
;; @004b                               store notrap aligned region1 v128, v39+24
;; @004b                               v129 = load.i64 notrap aligned region2 v127+8
;; @004b                               store notrap aligned region4 v129, v39+72
;; @004b                               v130 = load.i64 notrap aligned region2 v127+16
;; @004b                               store notrap aligned region5 v130, v39+64
;; @004b                               v131 = load.i64 notrap aligned region2 v127+24
;; @004b                               store notrap aligned region6 v131, v39+80
;; @004b                               v132 = iconst.i64 0
;; @004b                               v133 = iadd.i64 v72, v132  ; v132 = 0
;; @004b                               v134 = iconst.i32 4
;; @004b                               v135 = iconst.i64 32
;; @004b                               v136 = iadd v133, v135  ; v135 = 32
;; @004b                               store notrap aligned region2 v134, v136  ; v134 = 4
;; @004b                               v137 = iconst.i64 120
;; @004b                               v138 = iadd.i64 v72, v137  ; v137 = 120
;; @004b                               v139 = load.i64 notrap aligned region2 v138+8
;; @004b                               v140 = iconst.i32 0
;; @004b                               store notrap aligned region2 v140, v138  ; v140 = 0
;; @004b                               v141 = iconst.i32 0
;; @004b                               store notrap aligned region2 v141, v138+4  ; v141 = 0
;; @004b                               v142 = iconst.i64 0
;; @004b                               store notrap aligned region2 v142, v138+8  ; v142 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
