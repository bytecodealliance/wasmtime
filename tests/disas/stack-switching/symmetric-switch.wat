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
;; @003e                               v20 = iconst.i64 1
;; @003e                               v21 = iadd v18, v20  ; v20 = 1
;; @003e                               store notrap aligned v21, v15+72
;; @003e                               v23 = iconst.i64 48
;; @003e                               v24 = iadd v0, v23  ; v23 = 48
;; @003e                               v25 = load.i64 notrap aligned v0+8
;; @003e                               v26 = load.i64 notrap aligned v25+88
;; @003e                               v27 = load.i64 notrap aligned v25+96
;; @003e                               jump block2(v26, v27)
;;
;;                                 block2(v28: i64, v29: i64):
;; @003e                               v30 = iconst.i64 1
;; @003e                               v31 = icmp eq v28, v30  ; v30 = 1
;; @003e                               trapnz v31, user22
;; @003e                               jump block3
;;
;;                                 block3:
;; @003e                               v32 = load.i64 notrap aligned v29+48
;; @003e                               v33 = load.i64 notrap aligned v29+56
;; @003e                               v34 = iconst.i64 24
;; @003e                               v35 = iadd v33, v34  ; v34 = 24
;; @003e                               v36 = load.i64 notrap aligned v35+8
;; @003e                               v37 = load.i32 notrap aligned v33+40
;; @003e                               v38 = load.i32 notrap aligned v35
;; @003e                               jump block4(v37)
;;
;;                                 block4(v39: i32):
;; @003e                               v40 = icmp ult v39, v38
;; @003e                               brif v40, block5, block2(v32, v33)
;;
;;                                 block5:
;;                                     v135 = iconst.i32 8
;; @003e                               v41 = imul.i32 v39, v135  ; v135 = 8
;; @003e                               v42 = uextend.i64 v41
;; @003e                               v43 = iadd.i64 v36, v42
;; @003e                               v44 = load.i64 notrap aligned v43
;; @003e                               v45 = icmp eq v44, v24
;; @003e                               v46 = iconst.i32 1
;; @003e                               v47 = iadd.i32 v39, v46  ; v46 = 1
;; @003e                               brif v45, block6, block4(v47)
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned v29, v27+64
;; @003e                               v48 = iconst.i64 120
;; @003e                               v49 = iadd.i64 v27, v48  ; v48 = 120
;; @003e                               v50 = iconst.i64 0
;; @003e                               v51 = iadd.i64 v27, v50  ; v50 = 0
;; @003e                               v52 = iconst.i32 3
;; @003e                               v53 = iconst.i64 16
;; @003e                               v54 = iadd v51, v53  ; v53 = 16
;; @003e                               store notrap aligned v52, v54  ; v52 = 3
;; @003e                               v55 = iconst.i64 0
;; @003e                               v56 = iconst.i64 0
;; @003e                               store notrap aligned v55, v29+48  ; v55 = 0
;; @003e                               store notrap aligned v56, v29+56  ; v56 = 0
;; @003e                               v57 = load.i64 notrap aligned readonly v0+8
;; @003e                               v58 = iconst.i64 0
;; @003e                               v59 = iadd v51, v58  ; v58 = 0
;; @003e                               v60 = load.i64 notrap aligned v57+72
;; @003e                               store notrap aligned v60, v59+8
;; @003e                               v61 = load.i64 notrap aligned v27+72
;; @003e                               v62 = uextend.i128 v27
;; @003e                               v63 = uextend.i128 v61
;;                                     v133 = iconst.i64 64
;;                                     v134 = uextend.i128 v133  ; v133 = 64
;; @003e                               v64 = ishl v63, v134
;; @003e                               v65 = bor v64, v62
;; @003e                               v67 = iconst.i64 0
;; @003e                               v68 = iadd.i64 v15, v67  ; v67 = 0
;; @003e                               v69 = iconst.i64 16
;; @003e                               v70 = iadd v68, v69  ; v69 = 16
;; @003e                               v71 = load.i32 notrap aligned v70
;; @003e                               v72 = iconst.i32 0
;; @003e                               v73 = icmp ne v71, v72  ; v72 = 0
;; @003e                               brif v73, block9, block8
;;
;;                                 block8:
;; @003e                               v74 = iconst.i64 104
;; @003e                               v75 = iadd.i64 v15, v74  ; v74 = 104
;; @003e                               v76 = load.i64 notrap aligned v75+8
;; @003e                               v77 = load.i32 notrap aligned v75
;; @003e                               v78 = iconst.i32 1
;; @003e                               v79 = iadd v77, v78  ; v78 = 1
;; @003e                               store notrap aligned v79, v75
;; @003e                               v80 = uextend.i64 v77
;;                                     v132 = iconst.i64 16
;; @003e                               v81 = imul v80, v132  ; v132 = 16
;; @003e                               v82 = iadd v76, v81
;; @003e                               jump block10(v82)
;;
;;                                 block9:
;; @003e                               v83 = iconst.i64 120
;; @003e                               v84 = iadd.i64 v15, v83  ; v83 = 120
;; @003e                               v85 = load.i64 notrap aligned v84+8
;; @003e                               v86 = load.i32 notrap aligned v84
;; @003e                               v87 = iconst.i32 1
;; @003e                               v88 = iadd v86, v87  ; v87 = 1
;; @003e                               store notrap aligned v88, v84
;; @003e                               v89 = uextend.i64 v86
;;                                     v131 = iconst.i64 16
;; @003e                               v90 = imul v89, v131  ; v131 = 16
;; @003e                               v91 = iadd v85, v90
;; @003e                               jump block10(v91)
;;
;;                                 block10(v66: i64):
;; @003e                               store.i128 notrap aligned v65, v66
;; @003e                               v92 = iconst.i64 0
;; @003e                               v93 = iadd.i64 v15, v92  ; v92 = 0
;; @003e                               v94 = iconst.i32 1
;; @003e                               v95 = iconst.i64 16
;; @003e                               v96 = iadd v93, v95  ; v95 = 16
;; @003e                               store notrap aligned v94, v96  ; v94 = 1
;; @003e                               v97 = load.i64 notrap aligned v15+64
;; @003e                               store.i64 notrap aligned v32, v97+48
;; @003e                               store.i64 notrap aligned v33, v97+56
;; @003e                               v98 = iconst.i64 2
;; @003e                               v99 = load.i64 notrap aligned v0+8
;; @003e                               store notrap aligned v98, v99+88  ; v98 = 2
;; @003e                               store.i64 notrap aligned v15, v99+96
;; @003e                               v100 = iconst.i64 0
;; @003e                               v101 = iadd v93, v100  ; v100 = 0
;; @003e                               v102 = load.i64 notrap aligned v101
;; @003e                               store notrap aligned v102, v57+24
;; @003e                               v103 = load.i64 notrap aligned v101+8
;; @003e                               store notrap aligned v103, v57+72
;; @003e                               v104 = iconst.i64 80
;; @003e                               v105 = iadd.i64 v29, v104  ; v104 = 80
;; @003e                               v106 = load.i64 notrap aligned v105
;; @003e                               v107 = iconst.i64 -24
;; @003e                               v108 = iadd v106, v107  ; v107 = -24
;; @003e                               v109 = iconst.i64 80
;; @003e                               v110 = iadd v97, v109  ; v109 = 80
;; @003e                               v111 = load.i64 notrap aligned v110
;; @003e                               v112 = iconst.i64 -24
;; @003e                               v113 = iadd v111, v112  ; v112 = -24
;; @003e                               v114 = stack_addr.i64 ss0
;; @003e                               v115 = load.i64 notrap aligned v113
;; @003e                               store notrap aligned v115, v114
;; @003e                               v116 = load.i64 notrap aligned v108
;; @003e                               store notrap aligned v116, v113
;; @003e                               v117 = load.i64 notrap aligned v113+8
;; @003e                               store notrap aligned v117, v114+8
;; @003e                               v118 = load.i64 notrap aligned v108+8
;; @003e                               store notrap aligned v118, v113+8
;; @003e                               v119 = load.i64 notrap aligned v113+16
;; @003e                               store notrap aligned v119, v114+16
;; @003e                               v120 = load.i64 notrap aligned v108+16
;; @003e                               store notrap aligned v120, v113+16
;; @003e                               v121 = iconst.i64 3
;;                                     v130 = iconst.i64 32
;; @003e                               v122 = ishl v121, v130  ; v121 = 3, v130 = 32
;; @003e                               v123 = stack_switch v108, v114, v122
;; @003e                               v124 = iconst.i64 120
;; @003e                               v125 = iadd.i64 v27, v124  ; v124 = 120
;; @003e                               v126 = load.i64 notrap aligned v125+8
;; @003e                               v127 = iconst.i32 0
;; @003e                               store notrap aligned v127, v125  ; v127 = 0
;; @003e                               v128 = iconst.i32 0
;; @003e                               store notrap aligned v128, v125+4  ; v128 = 0
;; @003e                               v129 = iconst.i64 0
;; @003e                               store notrap aligned v129, v125+8  ; v129 = 0
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
;; @004b                               v20 = iconst.i64 1
;; @004b                               v21 = iadd v18, v20  ; v20 = 1
;; @004b                               store notrap aligned v21, v15+72
;; @004b                               v22 = load.i64 notrap aligned v15+64
;; @004b                               v23 = load.i64 notrap aligned v0+8
;; @004b                               v24 = load.i64 notrap aligned v23+88
;; @004b                               v25 = load.i64 notrap aligned v23+96
;; @004b                               store notrap aligned v24, v22+48
;; @004b                               store notrap aligned v25, v22+56
;; @004b                               v26 = iconst.i64 0
;; @004b                               store notrap aligned v26, v15+64  ; v26 = 0
;; @004b                               v27 = iconst.i64 2
;; @004b                               v28 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v27, v28+88  ; v27 = 2
;; @004b                               store notrap aligned v15, v28+96
;; @004b                               v29 = iconst.i64 0
;; @004b                               v30 = iadd v15, v29  ; v29 = 0
;; @004b                               v31 = iconst.i32 1
;; @004b                               v32 = iconst.i64 16
;; @004b                               v33 = iadd v30, v32  ; v32 = 16
;; @004b                               store notrap aligned v31, v33  ; v31 = 1
;; @004b                               v34 = iconst.i32 2
;; @004b                               v35 = iconst.i64 16
;; @004b                               v36 = iadd v25, v35  ; v35 = 16
;; @004b                               store notrap aligned v34, v36  ; v34 = 2
;; @004b                               v37 = load.i64 notrap aligned readonly v0+8
;; @004b                               v38 = iconst.i64 0
;; @004b                               v39 = iadd v25, v38  ; v38 = 0
;; @004b                               v40 = load.i64 notrap aligned v37+72
;; @004b                               store notrap aligned v40, v39+8
;; @004b                               v41 = load.i64 notrap aligned v37+24
;; @004b                               store notrap aligned v41, v39
;; @004b                               v42 = iconst.i64 0
;; @004b                               v43 = iadd v30, v42  ; v42 = 0
;; @004b                               v44 = load.i64 notrap aligned v43
;; @004b                               store notrap aligned v44, v37+24
;; @004b                               v45 = load.i64 notrap aligned v43+8
;; @004b                               store notrap aligned v45, v37+72
;; @004b                               v46 = iconst.i64 24
;; @004b                               v47 = iadd v25, v46  ; v46 = 24
;; @004b                               v48 = iconst.i32 1
;; @004b                               v49 = stack_addr.i64 ss0
;; @004b                               store notrap aligned v48, v47+4  ; v48 = 1
;; @004b                               store notrap aligned v49, v47+8
;; @004b                               v51 = iconst.i64 48
;; @004b                               v52 = iadd.i64 v0, v51  ; v51 = 48
;; @004b                               v53 = iconst.i32 1
;; @004b                               v54 = load.i64 notrap aligned v47+8
;; @004b                               store notrap aligned v52, v54
;; @004b                               store notrap aligned v53, v47  ; v53 = 1
;; @004b                               v55 = iconst.i32 0
;; @004b                               store notrap aligned v55, v25+40  ; v55 = 0
;; @004b                               v56 = iconst.i64 1
;;                                     v108 = iconst.i64 32
;; @004b                               v57 = ishl v56, v108  ; v56 = 1, v108 = 32
;; @004b                               v58 = iconst.i64 80
;; @004b                               v59 = iadd v22, v58  ; v58 = 80
;; @004b                               v60 = load.i64 notrap aligned v59
;; @004b                               v61 = iconst.i64 -24
;; @004b                               v62 = iadd v60, v61  ; v61 = -24
;; @004b                               v63 = stack_switch v62, v62, v57
;; @004b                               v64 = load.i64 notrap aligned v0+8
;; @004b                               v65 = load.i64 notrap aligned v64+88
;; @004b                               v66 = load.i64 notrap aligned v64+96
;; @004b                               v67 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v24, v67+88
;; @004b                               store notrap aligned v25, v67+96
;; @004b                               v68 = iconst.i32 1
;; @004b                               v69 = iconst.i64 16
;; @004b                               v70 = iadd v25, v69  ; v69 = 16
;; @004b                               store notrap aligned v68, v70  ; v68 = 1
;; @004b                               v71 = iconst.i32 0
;; @004b                               store notrap aligned v71, v47  ; v71 = 0
;; @004b                               v72 = iconst.i32 0
;; @004b                               store notrap aligned v72, v47+4  ; v72 = 0
;; @004b                               v73 = iconst.i64 0
;; @004b                               store notrap aligned v73, v47+8  ; v73 = 0
;; @004b                               store notrap aligned v26, v25+40  ; v26 = 0
;;                                     v107 = iconst.i64 32
;; @004b                               v74 = ushr v63, v107  ; v107 = 32
;; @004b                               brif v74, block4, block3
;;
;;                                 block4:
;; @004b                               v75 = iconst.i64 0
;; @004b                               v76 = iadd.i64 v66, v75  ; v75 = 0
;; @004b                               v77 = iconst.i64 0
;; @004b                               v78 = iadd v76, v77  ; v77 = 0
;; @004b                               v79 = load.i64 notrap aligned v37+72
;; @004b                               store notrap aligned v79, v78+8
;; @004b                               v80 = iconst.i64 0
;; @004b                               v81 = iadd.i64 v25, v80  ; v80 = 0
;; @004b                               v82 = load.i64 notrap aligned v81
;; @004b                               store notrap aligned v82, v37+24
;; @004b                               v83 = load.i64 notrap aligned v81+8
;; @004b                               store notrap aligned v83, v37+72
;; @004b                               v84 = ireduce.i32 v63
;; @004b                               v85 = load.i64 notrap aligned v66+72
;; @004b                               v86 = uextend.i128 v66
;; @004b                               v87 = uextend.i128 v85
;;                                     v105 = iconst.i64 64
;;                                     v106 = uextend.i128 v105  ; v105 = 64
;; @004b                               v88 = ishl v87, v106
;; @004b                               v89 = bor v88, v86
;; @004b                               jump block5
;;
;;                                 block6 cold:
;; @004b                               trap user12
;;
;;                                 block5:
;; @004b                               br_table v84, block6, []
;;
;;                                 block3:
;; @004b                               v90 = iconst.i64 0
;; @004b                               v91 = iadd.i64 v25, v90  ; v90 = 0
;; @004b                               v92 = load.i64 notrap aligned v91
;; @004b                               store notrap aligned v92, v37+24
;; @004b                               v93 = load.i64 notrap aligned v91+8
;; @004b                               store notrap aligned v93, v37+72
;; @004b                               v94 = iconst.i64 0
;; @004b                               v95 = iadd.i64 v66, v94  ; v94 = 0
;; @004b                               v96 = iconst.i32 4
;; @004b                               v97 = iconst.i64 16
;; @004b                               v98 = iadd v95, v97  ; v97 = 16
;; @004b                               store notrap aligned v96, v98  ; v96 = 4
;; @004b                               v99 = iconst.i64 104
;; @004b                               v100 = iadd.i64 v66, v99  ; v99 = 104
;; @004b                               v101 = load.i64 notrap aligned v100+8
;; @004b                               v102 = iconst.i32 0
;; @004b                               store notrap aligned v102, v100  ; v102 = 0
;; @004b                               v103 = iconst.i32 0
;; @004b                               store notrap aligned v103, v100+4  ; v103 = 0
;; @004b                               v104 = iconst.i64 0
;; @004b                               store notrap aligned v104, v100+8  ; v104 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
