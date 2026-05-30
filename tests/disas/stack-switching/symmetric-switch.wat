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
;; @003c                               v12 = iconst.i64 64
;; @003c                               v13 = uextend.i128 v12  ; v12 = 64
;; @003c                               v14 = ishl v11, v13
;; @003c                               v15 = bor v14, v10
;; @003e                               v17 = ireduce.i64 v15
;;                                     v138 = iconst.i64 64
;;                                     v139 = uextend.i128 v138  ; v138 = 64
;; @003e                               v18 = ushr v15, v139
;; @003e                               v19 = ireduce.i64 v18
;; @003e                               trapz v17, user16
;; @003e                               v20 = load.i64 notrap aligned v17+72
;; @003e                               v21 = icmp eq v20, v19
;; @003e                               trapz v21, user23
;; @003e                               v22 = iconst.i64 1
;; @003e                               v23 = iadd v20, v22  ; v22 = 1
;; @003e                               store notrap aligned v23, v17+72
;; @003e                               v25 = iconst.i64 48
;; @003e                               v26 = iadd v0, v25  ; v25 = 48
;; @003e                               v27 = load.i64 notrap aligned v0+8
;; @003e                               v28 = load.i64 notrap aligned v27+88
;; @003e                               v29 = load.i64 notrap aligned v27+96
;; @003e                               jump block2(v28, v29)
;;
;;                                 block2(v30: i64, v31: i64):
;; @003e                               v32 = iconst.i64 1
;; @003e                               v33 = icmp eq v30, v32  ; v32 = 1
;; @003e                               trapnz v33, user22
;; @003e                               jump block3
;;
;;                                 block3:
;; @003e                               v34 = load.i64 notrap aligned v31+48
;; @003e                               v35 = load.i64 notrap aligned v31+56
;; @003e                               v36 = iconst.i64 24
;; @003e                               v37 = iadd v35, v36  ; v36 = 24
;; @003e                               v38 = load.i64 notrap aligned v37+8
;; @003e                               v39 = load.i32 notrap aligned v35+40
;; @003e                               v40 = load.i32 notrap aligned v37
;; @003e                               jump block4(v39)
;;
;;                                 block4(v41: i32):
;; @003e                               v42 = icmp ult v41, v40
;; @003e                               brif v42, block5, block2(v34, v35)
;;
;;                                 block5:
;; @003e                               v43 = iconst.i32 8
;; @003e                               v44 = imul.i32 v41, v43  ; v43 = 8
;; @003e                               v45 = uextend.i64 v44
;; @003e                               v46 = iadd.i64 v38, v45
;; @003e                               v47 = load.i64 notrap aligned v46
;; @003e                               v48 = icmp eq v47, v26
;; @003e                               v49 = iconst.i32 1
;; @003e                               v50 = iadd.i32 v41, v49  ; v49 = 1
;; @003e                               brif v48, block6, block4(v50)
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned v31, v29+64
;; @003e                               v51 = iconst.i64 120
;; @003e                               v52 = iadd.i64 v29, v51  ; v51 = 120
;; @003e                               v53 = iconst.i64 0
;; @003e                               v54 = iadd.i64 v29, v53  ; v53 = 0
;; @003e                               v55 = iconst.i32 3
;; @003e                               v56 = iconst.i64 16
;; @003e                               v57 = iadd v54, v56  ; v56 = 16
;; @003e                               store notrap aligned v55, v57  ; v55 = 3
;; @003e                               v58 = iconst.i64 0
;; @003e                               v59 = iconst.i64 0
;; @003e                               store notrap aligned v58, v31+48  ; v58 = 0
;; @003e                               store notrap aligned v59, v31+56  ; v59 = 0
;; @003e                               v60 = load.i64 notrap aligned readonly v0+8
;; @003e                               v61 = iconst.i64 0
;; @003e                               v62 = iadd v54, v61  ; v61 = 0
;; @003e                               v63 = load.i64 notrap aligned v60+72
;; @003e                               store notrap aligned v63, v62+8
;; @003e                               v64 = load.i64 notrap aligned v29+72
;; @003e                               v65 = uextend.i128 v29
;; @003e                               v66 = uextend.i128 v64
;; @003e                               v67 = iconst.i64 64
;; @003e                               v68 = uextend.i128 v67  ; v67 = 64
;; @003e                               v69 = ishl v66, v68
;; @003e                               v70 = bor v69, v65
;; @003e                               v72 = iconst.i64 0
;; @003e                               v73 = iadd.i64 v17, v72  ; v72 = 0
;; @003e                               v74 = iconst.i64 16
;; @003e                               v75 = iadd v73, v74  ; v74 = 16
;; @003e                               v76 = load.i32 notrap aligned v75
;; @003e                               v77 = iconst.i32 0
;; @003e                               v78 = icmp ne v76, v77  ; v77 = 0
;; @003e                               brif v78, block9, block8
;;
;;                                 block8:
;; @003e                               v79 = iconst.i64 104
;; @003e                               v80 = iadd.i64 v17, v79  ; v79 = 104
;; @003e                               v81 = load.i64 notrap aligned v80+8
;; @003e                               v82 = load.i32 notrap aligned v80
;; @003e                               v83 = iconst.i32 1
;; @003e                               v84 = iadd v82, v83  ; v83 = 1
;; @003e                               store notrap aligned v84, v80
;; @003e                               v85 = uextend.i64 v82
;; @003e                               v86 = iconst.i64 16
;; @003e                               v87 = imul v85, v86  ; v86 = 16
;; @003e                               v88 = iadd v81, v87
;; @003e                               jump block10(v88)
;;
;;                                 block9:
;; @003e                               v89 = iconst.i64 120
;; @003e                               v90 = iadd.i64 v17, v89  ; v89 = 120
;; @003e                               v91 = load.i64 notrap aligned v90+8
;; @003e                               v92 = load.i32 notrap aligned v90
;; @003e                               v93 = iconst.i32 1
;; @003e                               v94 = iadd v92, v93  ; v93 = 1
;; @003e                               store notrap aligned v94, v90
;; @003e                               v95 = uextend.i64 v92
;; @003e                               v96 = iconst.i64 16
;; @003e                               v97 = imul v95, v96  ; v96 = 16
;; @003e                               v98 = iadd v91, v97
;; @003e                               jump block10(v98)
;;
;;                                 block10(v71: i64):
;; @003e                               store.i128 notrap aligned v70, v71
;; @003e                               v99 = iconst.i64 0
;; @003e                               v100 = iadd.i64 v17, v99  ; v99 = 0
;; @003e                               v101 = iconst.i32 1
;; @003e                               v102 = iconst.i64 16
;; @003e                               v103 = iadd v100, v102  ; v102 = 16
;; @003e                               store notrap aligned v101, v103  ; v101 = 1
;; @003e                               v104 = load.i64 notrap aligned v17+64
;; @003e                               store.i64 notrap aligned v34, v104+48
;; @003e                               store.i64 notrap aligned v35, v104+56
;; @003e                               v105 = iconst.i64 2
;; @003e                               v106 = load.i64 notrap aligned v0+8
;; @003e                               store notrap aligned v105, v106+88  ; v105 = 2
;; @003e                               store.i64 notrap aligned v17, v106+96
;; @003e                               v107 = iconst.i64 0
;; @003e                               v108 = iadd v100, v107  ; v107 = 0
;; @003e                               v109 = load.i64 notrap aligned v108
;; @003e                               store notrap aligned v109, v60+24
;; @003e                               v110 = load.i64 notrap aligned v108+8
;; @003e                               store notrap aligned v110, v60+72
;; @003e                               v111 = iconst.i64 80
;; @003e                               v112 = iadd.i64 v31, v111  ; v111 = 80
;; @003e                               v113 = load.i64 notrap aligned v112
;; @003e                               v114 = iconst.i64 -24
;; @003e                               v115 = iadd v113, v114  ; v114 = -24
;; @003e                               v116 = iconst.i64 80
;; @003e                               v117 = iadd v104, v116  ; v116 = 80
;; @003e                               v118 = load.i64 notrap aligned v117
;; @003e                               v119 = iconst.i64 -24
;; @003e                               v120 = iadd v118, v119  ; v119 = -24
;; @003e                               v121 = stack_addr.i64 ss0
;; @003e                               v122 = load.i64 notrap aligned v120
;; @003e                               store notrap aligned v122, v121
;; @003e                               v123 = load.i64 notrap aligned v115
;; @003e                               store notrap aligned v123, v120
;; @003e                               v124 = load.i64 notrap aligned v120+8
;; @003e                               store notrap aligned v124, v121+8
;; @003e                               v125 = load.i64 notrap aligned v115+8
;; @003e                               store notrap aligned v125, v120+8
;; @003e                               v126 = load.i64 notrap aligned v120+16
;; @003e                               store notrap aligned v126, v121+16
;; @003e                               v127 = load.i64 notrap aligned v115+16
;; @003e                               store notrap aligned v127, v120+16
;; @003e                               v128 = iconst.i64 3
;; @003e                               v129 = iconst.i64 32
;; @003e                               v130 = ishl v128, v129  ; v128 = 3, v129 = 32
;; @003e                               v131 = stack_switch v115, v121, v130
;; @003e                               v132 = iconst.i64 120
;; @003e                               v133 = iadd.i64 v29, v132  ; v132 = 120
;; @003e                               v134 = load.i64 notrap aligned v133+8
;; @003e                               v135 = iconst.i32 0
;; @003e                               store notrap aligned v135, v133  ; v135 = 0
;; @003e                               v136 = iconst.i32 0
;; @003e                               store notrap aligned v136, v133+4  ; v136 = 0
;; @003e                               v137 = iconst.i64 0
;; @003e                               store notrap aligned v137, v133+8  ; v137 = 0
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
;; @0049                               v12 = iconst.i64 64
;; @0049                               v13 = uextend.i128 v12  ; v12 = 64
;; @0049                               v14 = ishl v11, v13
;; @0049                               v15 = bor v14, v10
;; @004b                               jump block2
;;
;;                                 block2:
;; @004b                               v17 = ireduce.i64 v15
;;                                     v111 = iconst.i64 64
;;                                     v112 = uextend.i128 v111  ; v111 = 64
;; @004b                               v18 = ushr.i128 v15, v112
;; @004b                               v19 = ireduce.i64 v18
;; @004b                               trapz v17, user16
;; @004b                               v20 = load.i64 notrap aligned v17+72
;; @004b                               v21 = icmp eq v20, v19
;; @004b                               trapz v21, user23
;; @004b                               v22 = iconst.i64 1
;; @004b                               v23 = iadd v20, v22  ; v22 = 1
;; @004b                               store notrap aligned v23, v17+72
;; @004b                               v24 = load.i64 notrap aligned v17+64
;; @004b                               v25 = load.i64 notrap aligned v0+8
;; @004b                               v26 = load.i64 notrap aligned v25+88
;; @004b                               v27 = load.i64 notrap aligned v25+96
;; @004b                               store notrap aligned v26, v24+48
;; @004b                               store notrap aligned v27, v24+56
;; @004b                               v28 = iconst.i64 0
;; @004b                               store notrap aligned v28, v17+64  ; v28 = 0
;; @004b                               v29 = iconst.i64 2
;; @004b                               v30 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v29, v30+88  ; v29 = 2
;; @004b                               store notrap aligned v17, v30+96
;; @004b                               v31 = iconst.i64 0
;; @004b                               v32 = iadd v17, v31  ; v31 = 0
;; @004b                               v33 = iconst.i32 1
;; @004b                               v34 = iconst.i64 16
;; @004b                               v35 = iadd v32, v34  ; v34 = 16
;; @004b                               store notrap aligned v33, v35  ; v33 = 1
;; @004b                               v36 = iconst.i32 2
;; @004b                               v37 = iconst.i64 16
;; @004b                               v38 = iadd v27, v37  ; v37 = 16
;; @004b                               store notrap aligned v36, v38  ; v36 = 2
;; @004b                               v39 = load.i64 notrap aligned readonly v0+8
;; @004b                               v40 = iconst.i64 0
;; @004b                               v41 = iadd v27, v40  ; v40 = 0
;; @004b                               v42 = load.i64 notrap aligned v39+72
;; @004b                               store notrap aligned v42, v41+8
;; @004b                               v43 = load.i64 notrap aligned v39+24
;; @004b                               store notrap aligned v43, v41
;; @004b                               v44 = iconst.i64 0
;; @004b                               v45 = iadd v32, v44  ; v44 = 0
;; @004b                               v46 = load.i64 notrap aligned v45
;; @004b                               store notrap aligned v46, v39+24
;; @004b                               v47 = load.i64 notrap aligned v45+8
;; @004b                               store notrap aligned v47, v39+72
;; @004b                               v48 = iconst.i64 24
;; @004b                               v49 = iadd v27, v48  ; v48 = 24
;; @004b                               v50 = iconst.i32 1
;; @004b                               v51 = stack_addr.i64 ss0
;; @004b                               store notrap aligned v50, v49+4  ; v50 = 1
;; @004b                               store notrap aligned v51, v49+8
;; @004b                               v53 = iconst.i64 48
;; @004b                               v54 = iadd.i64 v0, v53  ; v53 = 48
;; @004b                               v55 = iconst.i32 1
;; @004b                               v56 = load.i64 notrap aligned v49+8
;; @004b                               store notrap aligned v54, v56
;; @004b                               store notrap aligned v55, v49  ; v55 = 1
;; @004b                               v57 = iconst.i32 0
;; @004b                               store notrap aligned v57, v27+40  ; v57 = 0
;; @004b                               v58 = iconst.i64 1
;; @004b                               v59 = iconst.i64 32
;; @004b                               v60 = ishl v58, v59  ; v58 = 1, v59 = 32
;; @004b                               v61 = iconst.i64 80
;; @004b                               v62 = iadd v24, v61  ; v61 = 80
;; @004b                               v63 = load.i64 notrap aligned v62
;; @004b                               v64 = iconst.i64 -24
;; @004b                               v65 = iadd v63, v64  ; v64 = -24
;; @004b                               v66 = stack_switch v65, v65, v60
;; @004b                               v67 = load.i64 notrap aligned v0+8
;; @004b                               v68 = load.i64 notrap aligned v67+88
;; @004b                               v69 = load.i64 notrap aligned v67+96
;; @004b                               v70 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v26, v70+88
;; @004b                               store notrap aligned v27, v70+96
;; @004b                               v71 = iconst.i32 1
;; @004b                               v72 = iconst.i64 16
;; @004b                               v73 = iadd v27, v72  ; v72 = 16
;; @004b                               store notrap aligned v71, v73  ; v71 = 1
;; @004b                               v74 = iconst.i32 0
;; @004b                               store notrap aligned v74, v49  ; v74 = 0
;; @004b                               v75 = iconst.i32 0
;; @004b                               store notrap aligned v75, v49+4  ; v75 = 0
;; @004b                               v76 = iconst.i64 0
;; @004b                               store notrap aligned v76, v49+8  ; v76 = 0
;; @004b                               store notrap aligned v28, v27+40  ; v28 = 0
;;                                     v110 = iconst.i64 32
;; @004b                               v77 = ushr v66, v110  ; v110 = 32
;; @004b                               brif v77, block4, block3
;;
;;                                 block4:
;; @004b                               v78 = iconst.i64 0
;; @004b                               v79 = iadd.i64 v69, v78  ; v78 = 0
;; @004b                               v80 = iconst.i64 0
;; @004b                               v81 = iadd v79, v80  ; v80 = 0
;; @004b                               v82 = load.i64 notrap aligned v39+72
;; @004b                               store notrap aligned v82, v81+8
;; @004b                               v83 = iconst.i64 0
;; @004b                               v84 = iadd.i64 v27, v83  ; v83 = 0
;; @004b                               v85 = load.i64 notrap aligned v84
;; @004b                               store notrap aligned v85, v39+24
;; @004b                               v86 = load.i64 notrap aligned v84+8
;; @004b                               store notrap aligned v86, v39+72
;; @004b                               v87 = ireduce.i32 v66
;; @004b                               v88 = load.i64 notrap aligned v69+72
;; @004b                               v89 = uextend.i128 v69
;; @004b                               v90 = uextend.i128 v88
;; @004b                               v91 = iconst.i64 64
;; @004b                               v92 = uextend.i128 v91  ; v91 = 64
;; @004b                               v93 = ishl v90, v92
;; @004b                               v94 = bor v93, v89
;; @004b                               jump block5
;;
;;                                 block6 cold:
;; @004b                               trap user12
;;
;;                                 block5:
;; @004b                               br_table v87, block6, []
;;
;;                                 block3:
;; @004b                               v95 = iconst.i64 0
;; @004b                               v96 = iadd.i64 v27, v95  ; v95 = 0
;; @004b                               v97 = load.i64 notrap aligned v96
;; @004b                               store notrap aligned v97, v39+24
;; @004b                               v98 = load.i64 notrap aligned v96+8
;; @004b                               store notrap aligned v98, v39+72
;; @004b                               v99 = iconst.i64 0
;; @004b                               v100 = iadd.i64 v69, v99  ; v99 = 0
;; @004b                               v101 = iconst.i32 4
;; @004b                               v102 = iconst.i64 16
;; @004b                               v103 = iadd v100, v102  ; v102 = 16
;; @004b                               store notrap aligned v101, v103  ; v101 = 4
;; @004b                               v104 = iconst.i64 104
;; @004b                               v105 = iadd.i64 v69, v104  ; v104 = 104
;; @004b                               v106 = load.i64 notrap aligned v105+8
;; @004b                               v107 = iconst.i32 0
;; @004b                               store notrap aligned v107, v105  ; v107 = 0
;; @004b                               v108 = iconst.i32 0
;; @004b                               store notrap aligned v108, v105+4  ; v108 = 0
;; @004b                               v109 = iconst.i64 0
;; @004b                               store notrap aligned v109, v105+8  ; v109 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
