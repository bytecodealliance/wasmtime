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
;;     ss0 = explicit_slot 24, align = 256
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003a                               v2 = iconst.i32 1
;; @003a                               v4 = call fn0(v0, v2)  ; v2 = 1
;; @003c                               trapz v4, user16
;; @003c                               v5 = iconst.i32 1
;; @003c                               v6 = iconst.i32 0
;; @003c                               v8 = call fn1(v0, v4, v5, v6)  ; v5 = 1, v6 = 0
;; @003c                               v9 = load.i64 notrap aligned v8+72
;; @003c                               v10 = uextend.i128 v8
;; @003c                               v11 = uextend.i128 v9
;;                                     v138 = iconst.i64 64
;;                                     v139 = uextend.i128 v138  ; v138 = 64
;; @003c                               v12 = ishl v11, v139
;; @003c                               v13 = bor v12, v10
;; @003e                               v15 = ireduce.i64 v13
;;                                     v136 = iconst.i64 64
;;                                     v137 = uextend.i128 v136  ; v136 = 64
;; @003e                               v16 = ushr v13, v137
;; @003e                               v17 = ireduce.i64 v16
;; @003e                               trapz v15, user16
;; @003e                               v18 = load.i64 notrap aligned v15+72
;; @003e                               v19 = icmp eq v18, v17
;; @003e                               trapz v19, user23
;;                                     v135 = iconst.i64 1
;; @003e                               v20 = iadd v18, v135  ; v135 = 1
;; @003e                               store notrap aligned v20, v15+72
;;                                     v134 = iconst.i64 48
;; @003e                               v22 = iadd v0, v134  ; v134 = 48
;; @003e                               v23 = load.i64 notrap aligned v0+8
;; @003e                               v24 = load.i64 notrap aligned v23+88
;; @003e                               v25 = load.i64 notrap aligned v23+96
;; @003e                               jump block2(v24, v25)
;;
;;                                 block2(v26: i64, v27: i64):
;; @003e                               v28 = iconst.i64 1
;; @003e                               v29 = icmp eq v26, v28  ; v28 = 1
;; @003e                               trapnz v29, user22
;; @003e                               jump block3
;;
;;                                 block3:
;; @003e                               v30 = load.i64 notrap aligned v27+48
;; @003e                               v31 = load.i64 notrap aligned v27+56
;;                                     v133 = iconst.i64 24
;; @003e                               v32 = iadd v31, v133  ; v133 = 24
;; @003e                               v33 = load.i64 notrap aligned v32+8
;; @003e                               v34 = load.i32 notrap aligned v31+40
;; @003e                               v35 = load.i32 notrap aligned v32
;; @003e                               jump block4(v34)
;;
;;                                 block4(v36: i32):
;; @003e                               v37 = icmp ult v36, v35
;; @003e                               brif v37, block5, block2(v30, v31)
;;
;;                                 block5:
;;                                     v132 = iconst.i32 8
;; @003e                               v38 = imul.i32 v36, v132  ; v132 = 8
;; @003e                               v39 = uextend.i64 v38
;; @003e                               v40 = iadd.i64 v33, v39
;; @003e                               v41 = load.i64 notrap aligned v40
;; @003e                               v42 = icmp eq v41, v22
;;                                     v131 = iconst.i32 1
;; @003e                               v43 = iadd.i32 v36, v131  ; v131 = 1
;; @003e                               brif v42, block6, block4(v43)
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned v27, v25+64
;;                                     v130 = iconst.i64 120
;; @003e                               v44 = iadd.i64 v25, v130  ; v130 = 120
;;                                     v129 = iconst.i64 0
;; @003e                               v45 = iadd.i64 v25, v129  ; v129 = 0
;; @003e                               v46 = iconst.i32 3
;;                                     v128 = iconst.i64 16
;; @003e                               v47 = iadd v45, v128  ; v128 = 16
;; @003e                               store notrap aligned v46, v47  ; v46 = 3
;; @003e                               v48 = iconst.i64 0
;; @003e                               v49 = iconst.i64 0
;; @003e                               store notrap aligned v48, v27+48  ; v48 = 0
;; @003e                               store notrap aligned v49, v27+56  ; v49 = 0
;; @003e                               v50 = load.i64 notrap aligned readonly v0+8
;;                                     v127 = iconst.i64 0
;; @003e                               v51 = iadd v45, v127  ; v127 = 0
;; @003e                               v52 = load.i64 notrap aligned v50+72
;; @003e                               store notrap aligned v52, v51+8
;; @003e                               v53 = load.i64 notrap aligned v25+72
;; @003e                               v54 = uextend.i128 v25
;; @003e                               v55 = uextend.i128 v53
;;                                     v125 = iconst.i64 64
;;                                     v126 = uextend.i128 v125  ; v125 = 64
;; @003e                               v56 = ishl v55, v126
;; @003e                               v57 = bor v56, v54
;;                                     v124 = iconst.i64 0
;; @003e                               v59 = iadd.i64 v15, v124  ; v124 = 0
;;                                     v123 = iconst.i64 16
;; @003e                               v60 = iadd v59, v123  ; v123 = 16
;; @003e                               v61 = load.i32 notrap aligned v60
;; @003e                               v62 = iconst.i32 0
;; @003e                               v63 = icmp ne v61, v62  ; v62 = 0
;; @003e                               brif v63, block9, block8
;;
;;                                 block8:
;;                                     v122 = iconst.i64 104
;; @003e                               v64 = iadd.i64 v15, v122  ; v122 = 104
;; @003e                               v65 = load.i64 notrap aligned v64+8
;; @003e                               v66 = load.i32 notrap aligned v64
;;                                     v121 = iconst.i32 1
;; @003e                               v67 = iadd v66, v121  ; v121 = 1
;; @003e                               store notrap aligned v67, v64
;; @003e                               v68 = uextend.i64 v66
;;                                     v120 = iconst.i64 16
;; @003e                               v69 = imul v68, v120  ; v120 = 16
;; @003e                               v70 = iadd v65, v69
;; @003e                               jump block10(v70)
;;
;;                                 block9:
;;                                     v119 = iconst.i64 120
;; @003e                               v71 = iadd.i64 v15, v119  ; v119 = 120
;; @003e                               v72 = load.i64 notrap aligned v71+8
;; @003e                               v73 = load.i32 notrap aligned v71
;;                                     v118 = iconst.i32 1
;; @003e                               v74 = iadd v73, v118  ; v118 = 1
;; @003e                               store notrap aligned v74, v71
;; @003e                               v75 = uextend.i64 v73
;;                                     v117 = iconst.i64 16
;; @003e                               v76 = imul v75, v117  ; v117 = 16
;; @003e                               v77 = iadd v72, v76
;; @003e                               jump block10(v77)
;;
;;                                 block10(v58: i64):
;; @003e                               store.i128 notrap aligned v57, v58
;;                                     v116 = iconst.i64 0
;; @003e                               v78 = iadd.i64 v15, v116  ; v116 = 0
;; @003e                               v79 = iconst.i32 1
;;                                     v115 = iconst.i64 16
;; @003e                               v80 = iadd v78, v115  ; v115 = 16
;; @003e                               store notrap aligned v79, v80  ; v79 = 1
;; @003e                               v81 = load.i64 notrap aligned v15+64
;; @003e                               store.i64 notrap aligned v30, v81+48
;; @003e                               store.i64 notrap aligned v31, v81+56
;; @003e                               v82 = iconst.i64 2
;; @003e                               v83 = load.i64 notrap aligned v0+8
;; @003e                               store notrap aligned v82, v83+88  ; v82 = 2
;; @003e                               store.i64 notrap aligned v15, v83+96
;;                                     v114 = iconst.i64 0
;; @003e                               v84 = iadd v78, v114  ; v114 = 0
;; @003e                               v85 = load.i64 notrap aligned v84
;; @003e                               store notrap aligned v85, v50+24
;; @003e                               v86 = load.i64 notrap aligned v84+8
;; @003e                               store notrap aligned v86, v50+72
;;                                     v113 = iconst.i64 80
;; @003e                               v87 = iadd.i64 v27, v113  ; v113 = 80
;; @003e                               v88 = load.i64 notrap aligned v87
;;                                     v112 = iconst.i64 -24
;; @003e                               v89 = iadd v88, v112  ; v112 = -24
;;                                     v111 = iconst.i64 80
;; @003e                               v90 = iadd v81, v111  ; v111 = 80
;; @003e                               v91 = load.i64 notrap aligned v90
;;                                     v110 = iconst.i64 -24
;; @003e                               v92 = iadd v91, v110  ; v110 = -24
;; @003e                               v93 = stack_addr.i64 ss0
;; @003e                               v94 = load.i64 notrap aligned v92
;; @003e                               store notrap aligned v94, v93
;; @003e                               v95 = load.i64 notrap aligned v89
;; @003e                               store notrap aligned v95, v92
;; @003e                               v96 = load.i64 notrap aligned v92+8
;; @003e                               store notrap aligned v96, v93+8
;; @003e                               v97 = load.i64 notrap aligned v89+8
;; @003e                               store notrap aligned v97, v92+8
;; @003e                               v98 = load.i64 notrap aligned v92+16
;; @003e                               store notrap aligned v98, v93+16
;; @003e                               v99 = load.i64 notrap aligned v89+16
;; @003e                               store notrap aligned v99, v92+16
;; @003e                               v100 = iconst.i64 3
;;                                     v109 = iconst.i64 32
;; @003e                               v101 = ishl v100, v109  ; v100 = 3, v109 = 32
;; @003e                               v102 = stack_switch v89, v93, v101
;;                                     v108 = iconst.i64 120
;; @003e                               v103 = iadd.i64 v25, v108  ; v108 = 120
;; @003e                               v104 = load.i64 notrap aligned v103+8
;; @003e                               v105 = iconst.i32 0
;; @003e                               store notrap aligned v105, v103  ; v105 = 0
;; @003e                               v106 = iconst.i32 0
;; @003e                               store notrap aligned v106, v103+4  ; v106 = 0
;; @003e                               v107 = iconst.i64 0
;; @003e                               store notrap aligned v107, v103+8  ; v107 = 0
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i128) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0047                               v2 = iconst.i32 0
;; @0047                               v4 = call fn0(v0, v2)  ; v2 = 0
;; @0049                               trapz v4, user16
;; @0049                               v5 = iconst.i32 0
;; @0049                               v6 = iconst.i32 0
;; @0049                               v8 = call fn1(v0, v4, v5, v6)  ; v5 = 0, v6 = 0
;; @0049                               v9 = load.i64 notrap aligned v8+72
;; @0049                               v10 = uextend.i128 v8
;; @0049                               v11 = uextend.i128 v9
;;                                     v111 = iconst.i64 64
;;                                     v112 = uextend.i128 v111  ; v111 = 64
;; @0049                               v12 = ishl v11, v112
;; @0049                               v13 = bor v12, v10
;; @004b                               jump block2
;;
;;                                 block2:
;; @004b                               v15 = ireduce.i64 v13
;;                                     v109 = iconst.i64 64
;;                                     v110 = uextend.i128 v109  ; v109 = 64
;; @004b                               v16 = ushr.i128 v13, v110
;; @004b                               v17 = ireduce.i64 v16
;; @004b                               trapz v15, user16
;; @004b                               v18 = load.i64 notrap aligned v15+72
;; @004b                               v19 = icmp eq v18, v17
;; @004b                               trapz v19, user23
;;                                     v108 = iconst.i64 1
;; @004b                               v20 = iadd v18, v108  ; v108 = 1
;; @004b                               store notrap aligned v20, v15+72
;; @004b                               v21 = load.i64 notrap aligned v15+64
;; @004b                               v22 = load.i64 notrap aligned v0+8
;; @004b                               v23 = load.i64 notrap aligned v22+88
;; @004b                               v24 = load.i64 notrap aligned v22+96
;; @004b                               store notrap aligned v23, v21+48
;; @004b                               store notrap aligned v24, v21+56
;; @004b                               v25 = iconst.i64 0
;; @004b                               store notrap aligned v25, v15+64  ; v25 = 0
;; @004b                               v26 = iconst.i64 2
;; @004b                               v27 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v26, v27+88  ; v26 = 2
;; @004b                               store notrap aligned v15, v27+96
;;                                     v107 = iconst.i64 0
;; @004b                               v28 = iadd v15, v107  ; v107 = 0
;; @004b                               v29 = iconst.i32 1
;;                                     v106 = iconst.i64 16
;; @004b                               v30 = iadd v28, v106  ; v106 = 16
;; @004b                               store notrap aligned v29, v30  ; v29 = 1
;; @004b                               v31 = iconst.i32 2
;;                                     v105 = iconst.i64 16
;; @004b                               v32 = iadd v24, v105  ; v105 = 16
;; @004b                               store notrap aligned v31, v32  ; v31 = 2
;; @004b                               v33 = load.i64 notrap aligned readonly v0+8
;;                                     v104 = iconst.i64 0
;; @004b                               v34 = iadd v24, v104  ; v104 = 0
;; @004b                               v35 = load.i64 notrap aligned v33+72
;; @004b                               store notrap aligned v35, v34+8
;; @004b                               v36 = load.i64 notrap aligned v33+24
;; @004b                               store notrap aligned v36, v34
;;                                     v103 = iconst.i64 0
;; @004b                               v37 = iadd v28, v103  ; v103 = 0
;; @004b                               v38 = load.i64 notrap aligned v37
;; @004b                               store notrap aligned v38, v33+24
;; @004b                               v39 = load.i64 notrap aligned v37+8
;; @004b                               store notrap aligned v39, v33+72
;;                                     v102 = iconst.i64 24
;; @004b                               v40 = iadd v24, v102  ; v102 = 24
;; @004b                               v41 = iconst.i32 1
;; @004b                               v42 = stack_addr.i64 ss0
;; @004b                               store notrap aligned v41, v40+4  ; v41 = 1
;; @004b                               store notrap aligned v42, v40+8
;;                                     v101 = iconst.i64 48
;; @004b                               v44 = iadd.i64 v0, v101  ; v101 = 48
;; @004b                               v45 = iconst.i32 1
;; @004b                               v46 = load.i64 notrap aligned v40+8
;; @004b                               store notrap aligned v44, v46
;; @004b                               store notrap aligned v45, v40  ; v45 = 1
;; @004b                               v47 = iconst.i32 0
;; @004b                               store notrap aligned v47, v24+40  ; v47 = 0
;; @004b                               v48 = iconst.i64 1
;;                                     v100 = iconst.i64 32
;; @004b                               v49 = ishl v48, v100  ; v48 = 1, v100 = 32
;;                                     v99 = iconst.i64 80
;; @004b                               v50 = iadd v21, v99  ; v99 = 80
;; @004b                               v51 = load.i64 notrap aligned v50
;;                                     v98 = iconst.i64 -24
;; @004b                               v52 = iadd v51, v98  ; v98 = -24
;; @004b                               v53 = stack_switch v52, v52, v49
;; @004b                               v54 = load.i64 notrap aligned v0+8
;; @004b                               v55 = load.i64 notrap aligned v54+88
;; @004b                               v56 = load.i64 notrap aligned v54+96
;; @004b                               v57 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v23, v57+88
;; @004b                               store notrap aligned v24, v57+96
;; @004b                               v58 = iconst.i32 1
;;                                     v97 = iconst.i64 16
;; @004b                               v59 = iadd v24, v97  ; v97 = 16
;; @004b                               store notrap aligned v58, v59  ; v58 = 1
;; @004b                               v60 = iconst.i32 0
;; @004b                               store notrap aligned v60, v40  ; v60 = 0
;; @004b                               v61 = iconst.i32 0
;; @004b                               store notrap aligned v61, v40+4  ; v61 = 0
;; @004b                               v62 = iconst.i64 0
;; @004b                               store notrap aligned v62, v40+8  ; v62 = 0
;; @004b                               store notrap aligned v25, v24+40  ; v25 = 0
;;                                     v96 = iconst.i64 32
;; @004b                               v63 = ushr v53, v96  ; v96 = 32
;; @004b                               brif v63, block4, block3
;;
;;                                 block4:
;;                                     v95 = iconst.i64 0
;; @004b                               v64 = iadd.i64 v56, v95  ; v95 = 0
;;                                     v94 = iconst.i64 0
;; @004b                               v65 = iadd v64, v94  ; v94 = 0
;; @004b                               v66 = load.i64 notrap aligned v33+72
;; @004b                               store notrap aligned v66, v65+8
;;                                     v93 = iconst.i64 0
;; @004b                               v67 = iadd.i64 v24, v93  ; v93 = 0
;; @004b                               v68 = load.i64 notrap aligned v67
;; @004b                               store notrap aligned v68, v33+24
;; @004b                               v69 = load.i64 notrap aligned v67+8
;; @004b                               store notrap aligned v69, v33+72
;; @004b                               v70 = ireduce.i32 v53
;; @004b                               v71 = load.i64 notrap aligned v56+72
;; @004b                               v72 = uextend.i128 v56
;; @004b                               v73 = uextend.i128 v71
;;                                     v91 = iconst.i64 64
;;                                     v92 = uextend.i128 v91  ; v91 = 64
;; @004b                               v74 = ishl v73, v92
;; @004b                               v75 = bor v74, v72
;; @004b                               jump block5
;;
;;                                 block6 cold:
;; @004b                               trap user12
;;
;;                                 block5:
;; @004b                               br_table v70, block6, []
;;
;;                                 block3:
;;                                     v90 = iconst.i64 0
;; @004b                               v76 = iadd.i64 v24, v90  ; v90 = 0
;; @004b                               v77 = load.i64 notrap aligned v76
;; @004b                               store notrap aligned v77, v33+24
;; @004b                               v78 = load.i64 notrap aligned v76+8
;; @004b                               store notrap aligned v78, v33+72
;;                                     v89 = iconst.i64 0
;; @004b                               v79 = iadd.i64 v56, v89  ; v89 = 0
;; @004b                               v80 = iconst.i32 4
;;                                     v88 = iconst.i64 16
;; @004b                               v81 = iadd v79, v88  ; v88 = 16
;; @004b                               store notrap aligned v80, v81  ; v80 = 4
;;                                     v87 = iconst.i64 104
;; @004b                               v82 = iadd.i64 v56, v87  ; v87 = 104
;; @004b                               v83 = load.i64 notrap aligned v82+8
;; @004b                               v84 = iconst.i32 0
;; @004b                               store notrap aligned v84, v82  ; v84 = 0
;; @004b                               v85 = iconst.i32 0
;; @004b                               store notrap aligned v85, v82+4  ; v85 = 0
;; @004b                               v86 = iconst.i64 0
;; @004b                               store notrap aligned v86, v82+8  ; v86 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
