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
;; @003e                               v18 = iconst.i64 64
;; @003e                               v19 = uextend.i128 v18  ; v18 = 64
;; @003e                               v20 = ushr v15, v19
;; @003e                               v21 = ireduce.i64 v20
;; @003e                               trapz v17, user16
;; @003e                               v22 = load.i64 notrap aligned v17+72
;; @003e                               v23 = icmp eq v22, v21
;; @003e                               trapz v23, user23
;; @003e                               v24 = iconst.i64 1
;; @003e                               v25 = iadd v22, v24  ; v24 = 1
;; @003e                               store notrap aligned v25, v17+72
;; @003e                               v27 = iconst.i64 48
;; @003e                               v28 = iadd v0, v27  ; v27 = 48
;; @003e                               v29 = load.i64 notrap aligned v0+8
;; @003e                               v30 = load.i64 notrap aligned v29+88
;; @003e                               v31 = load.i64 notrap aligned v29+96
;; @003e                               jump block2(v30, v31)
;;
;;                                 block2(v32: i64, v33: i64):
;; @003e                               v34 = iconst.i64 1
;; @003e                               v35 = icmp eq v32, v34  ; v34 = 1
;; @003e                               trapnz v35, user22
;; @003e                               jump block3
;;
;;                                 block3:
;; @003e                               v36 = load.i64 notrap aligned v33+48
;; @003e                               v37 = load.i64 notrap aligned v33+56
;; @003e                               v38 = iconst.i64 24
;; @003e                               v39 = iadd v37, v38  ; v38 = 24
;; @003e                               v40 = load.i64 notrap aligned v39+8
;; @003e                               v41 = load.i32 notrap aligned v37+40
;; @003e                               v42 = load.i32 notrap aligned v39
;; @003e                               jump block4(v41)
;;
;;                                 block4(v43: i32):
;; @003e                               v44 = icmp ult v43, v42
;; @003e                               brif v44, block5, block2(v36, v37)
;;
;;                                 block5:
;; @003e                               v45 = iconst.i32 8
;; @003e                               v46 = imul.i32 v43, v45  ; v45 = 8
;; @003e                               v47 = uextend.i64 v46
;; @003e                               v48 = iadd.i64 v40, v47
;; @003e                               v49 = load.i64 notrap aligned v48
;; @003e                               v50 = icmp eq v49, v28
;; @003e                               v51 = iconst.i32 1
;; @003e                               v52 = iadd.i32 v43, v51  ; v51 = 1
;; @003e                               brif v50, block6, block4(v52)
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned v33, v31+64
;; @003e                               v53 = iconst.i64 120
;; @003e                               v54 = iadd.i64 v31, v53  ; v53 = 120
;; @003e                               v55 = iconst.i64 0
;; @003e                               v56 = iadd.i64 v31, v55  ; v55 = 0
;; @003e                               v57 = iconst.i32 3
;; @003e                               v58 = iconst.i64 16
;; @003e                               v59 = iadd v56, v58  ; v58 = 16
;; @003e                               store notrap aligned v57, v59  ; v57 = 3
;; @003e                               v60 = iconst.i64 0
;; @003e                               v61 = iconst.i64 0
;; @003e                               store notrap aligned v60, v33+48  ; v60 = 0
;; @003e                               store notrap aligned v61, v33+56  ; v61 = 0
;; @003e                               v62 = load.i64 notrap aligned readonly v0+8
;; @003e                               v63 = iconst.i64 0
;; @003e                               v64 = iadd v56, v63  ; v63 = 0
;; @003e                               v65 = load.i64 notrap aligned v62+72
;; @003e                               store notrap aligned v65, v64+8
;; @003e                               v66 = load.i64 notrap aligned v31+72
;; @003e                               v67 = uextend.i128 v31
;; @003e                               v68 = uextend.i128 v66
;; @003e                               v69 = iconst.i64 64
;; @003e                               v70 = uextend.i128 v69  ; v69 = 64
;; @003e                               v71 = ishl v68, v70
;; @003e                               v72 = bor v71, v67
;; @003e                               v74 = iconst.i64 0
;; @003e                               v75 = iadd.i64 v17, v74  ; v74 = 0
;; @003e                               v76 = iconst.i64 16
;; @003e                               v77 = iadd v75, v76  ; v76 = 16
;; @003e                               v78 = load.i32 notrap aligned v77
;; @003e                               v79 = iconst.i32 0
;; @003e                               v80 = icmp ne v78, v79  ; v79 = 0
;; @003e                               brif v80, block9, block8
;;
;;                                 block8:
;; @003e                               v81 = iconst.i64 104
;; @003e                               v82 = iadd.i64 v17, v81  ; v81 = 104
;; @003e                               v83 = load.i64 notrap aligned v82+8
;; @003e                               v84 = load.i32 notrap aligned v82
;; @003e                               v85 = iconst.i32 1
;; @003e                               v86 = iadd v84, v85  ; v85 = 1
;; @003e                               store notrap aligned v86, v82
;; @003e                               v87 = uextend.i64 v84
;; @003e                               v88 = iconst.i64 16
;; @003e                               v89 = imul v87, v88  ; v88 = 16
;; @003e                               v90 = iadd v83, v89
;; @003e                               jump block10(v90)
;;
;;                                 block9:
;; @003e                               v91 = iconst.i64 120
;; @003e                               v92 = iadd.i64 v17, v91  ; v91 = 120
;; @003e                               v93 = load.i64 notrap aligned v92+8
;; @003e                               v94 = load.i32 notrap aligned v92
;; @003e                               v95 = iconst.i32 1
;; @003e                               v96 = iadd v94, v95  ; v95 = 1
;; @003e                               store notrap aligned v96, v92
;; @003e                               v97 = uextend.i64 v94
;; @003e                               v98 = iconst.i64 16
;; @003e                               v99 = imul v97, v98  ; v98 = 16
;; @003e                               v100 = iadd v93, v99
;; @003e                               jump block10(v100)
;;
;;                                 block10(v73: i64):
;; @003e                               store.i128 notrap aligned v72, v73
;; @003e                               v101 = iconst.i64 0
;; @003e                               v102 = iadd.i64 v17, v101  ; v101 = 0
;; @003e                               v103 = iconst.i32 1
;; @003e                               v104 = iconst.i64 16
;; @003e                               v105 = iadd v102, v104  ; v104 = 16
;; @003e                               store notrap aligned v103, v105  ; v103 = 1
;; @003e                               v106 = load.i64 notrap aligned v17+64
;; @003e                               store.i64 notrap aligned v36, v106+48
;; @003e                               store.i64 notrap aligned v37, v106+56
;; @003e                               v107 = iconst.i64 2
;; @003e                               v108 = load.i64 notrap aligned v0+8
;; @003e                               store notrap aligned v107, v108+88  ; v107 = 2
;; @003e                               store.i64 notrap aligned v17, v108+96
;; @003e                               v109 = iconst.i64 0
;; @003e                               v110 = iadd v102, v109  ; v109 = 0
;; @003e                               v111 = load.i64 notrap aligned v110
;; @003e                               store notrap aligned v111, v62+24
;; @003e                               v112 = load.i64 notrap aligned v110+8
;; @003e                               store notrap aligned v112, v62+72
;; @003e                               v113 = iconst.i64 80
;; @003e                               v114 = iadd.i64 v33, v113  ; v113 = 80
;; @003e                               v115 = load.i64 notrap aligned v114
;; @003e                               v116 = iconst.i64 -24
;; @003e                               v117 = iadd v115, v116  ; v116 = -24
;; @003e                               v118 = iconst.i64 80
;; @003e                               v119 = iadd v106, v118  ; v118 = 80
;; @003e                               v120 = load.i64 notrap aligned v119
;; @003e                               v121 = iconst.i64 -24
;; @003e                               v122 = iadd v120, v121  ; v121 = -24
;; @003e                               v123 = stack_addr.i64 ss0
;; @003e                               v124 = load.i64 notrap aligned v122
;; @003e                               store notrap aligned v124, v123
;; @003e                               v125 = load.i64 notrap aligned v117
;; @003e                               store notrap aligned v125, v122
;; @003e                               v126 = load.i64 notrap aligned v122+8
;; @003e                               store notrap aligned v126, v123+8
;; @003e                               v127 = load.i64 notrap aligned v117+8
;; @003e                               store notrap aligned v127, v122+8
;; @003e                               v128 = load.i64 notrap aligned v122+16
;; @003e                               store notrap aligned v128, v123+16
;; @003e                               v129 = load.i64 notrap aligned v117+16
;; @003e                               store notrap aligned v129, v122+16
;; @003e                               v130 = iconst.i64 3
;; @003e                               v131 = iconst.i64 32
;; @003e                               v132 = ishl v130, v131  ; v130 = 3, v131 = 32
;; @003e                               v133 = stack_switch v117, v123, v132
;; @003e                               v134 = iconst.i64 120
;; @003e                               v135 = iadd.i64 v31, v134  ; v134 = 120
;; @003e                               v136 = load.i64 notrap aligned v135+8
;; @003e                               v137 = iconst.i32 0
;; @003e                               store notrap aligned v137, v135  ; v137 = 0
;; @003e                               v138 = iconst.i32 0
;; @003e                               store notrap aligned v138, v135+4  ; v138 = 0
;; @003e                               v139 = iconst.i64 0
;; @003e                               store notrap aligned v139, v135+8  ; v139 = 0
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
;; @004b                               v18 = iconst.i64 64
;; @004b                               v19 = uextend.i128 v18  ; v18 = 64
;; @004b                               v20 = ushr.i128 v15, v19
;; @004b                               v21 = ireduce.i64 v20
;; @004b                               trapz v17, user16
;; @004b                               v22 = load.i64 notrap aligned v17+72
;; @004b                               v23 = icmp eq v22, v21
;; @004b                               trapz v23, user23
;; @004b                               v24 = iconst.i64 1
;; @004b                               v25 = iadd v22, v24  ; v24 = 1
;; @004b                               store notrap aligned v25, v17+72
;; @004b                               v26 = load.i64 notrap aligned v17+64
;; @004b                               v27 = load.i64 notrap aligned v0+8
;; @004b                               v28 = load.i64 notrap aligned v27+88
;; @004b                               v29 = load.i64 notrap aligned v27+96
;; @004b                               store notrap aligned v28, v26+48
;; @004b                               store notrap aligned v29, v26+56
;; @004b                               v30 = iconst.i64 0
;; @004b                               store notrap aligned v30, v17+64  ; v30 = 0
;; @004b                               v31 = iconst.i64 2
;; @004b                               v32 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v31, v32+88  ; v31 = 2
;; @004b                               store notrap aligned v17, v32+96
;; @004b                               v33 = iconst.i64 0
;; @004b                               v34 = iadd v17, v33  ; v33 = 0
;; @004b                               v35 = iconst.i32 1
;; @004b                               v36 = iconst.i64 16
;; @004b                               v37 = iadd v34, v36  ; v36 = 16
;; @004b                               store notrap aligned v35, v37  ; v35 = 1
;; @004b                               v38 = iconst.i32 2
;; @004b                               v39 = iconst.i64 16
;; @004b                               v40 = iadd v29, v39  ; v39 = 16
;; @004b                               store notrap aligned v38, v40  ; v38 = 2
;; @004b                               v41 = load.i64 notrap aligned readonly v0+8
;; @004b                               v42 = iconst.i64 0
;; @004b                               v43 = iadd v29, v42  ; v42 = 0
;; @004b                               v44 = load.i64 notrap aligned v41+72
;; @004b                               store notrap aligned v44, v43+8
;; @004b                               v45 = load.i64 notrap aligned v41+24
;; @004b                               store notrap aligned v45, v43
;; @004b                               v46 = iconst.i64 0
;; @004b                               v47 = iadd v34, v46  ; v46 = 0
;; @004b                               v48 = load.i64 notrap aligned v47
;; @004b                               store notrap aligned v48, v41+24
;; @004b                               v49 = load.i64 notrap aligned v47+8
;; @004b                               store notrap aligned v49, v41+72
;; @004b                               v50 = iconst.i64 24
;; @004b                               v51 = iadd v29, v50  ; v50 = 24
;; @004b                               v52 = iconst.i32 1
;; @004b                               v53 = stack_addr.i64 ss0
;; @004b                               store notrap aligned v52, v51+4  ; v52 = 1
;; @004b                               store notrap aligned v53, v51+8
;; @004b                               v55 = iconst.i64 48
;; @004b                               v56 = iadd.i64 v0, v55  ; v55 = 48
;; @004b                               v57 = iconst.i32 1
;; @004b                               v58 = load.i64 notrap aligned v51+8
;; @004b                               store notrap aligned v56, v58
;; @004b                               store notrap aligned v57, v51  ; v57 = 1
;; @004b                               v59 = iconst.i32 0
;; @004b                               store notrap aligned v59, v29+40  ; v59 = 0
;; @004b                               v60 = iconst.i64 1
;; @004b                               v61 = iconst.i64 32
;; @004b                               v62 = ishl v60, v61  ; v60 = 1, v61 = 32
;; @004b                               v63 = iconst.i64 80
;; @004b                               v64 = iadd v26, v63  ; v63 = 80
;; @004b                               v65 = load.i64 notrap aligned v64
;; @004b                               v66 = iconst.i64 -24
;; @004b                               v67 = iadd v65, v66  ; v66 = -24
;; @004b                               v68 = stack_switch v67, v67, v62
;; @004b                               v69 = load.i64 notrap aligned v0+8
;; @004b                               v70 = load.i64 notrap aligned v69+88
;; @004b                               v71 = load.i64 notrap aligned v69+96
;; @004b                               v72 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v28, v72+88
;; @004b                               store notrap aligned v29, v72+96
;; @004b                               v73 = iconst.i32 1
;; @004b                               v74 = iconst.i64 16
;; @004b                               v75 = iadd v29, v74  ; v74 = 16
;; @004b                               store notrap aligned v73, v75  ; v73 = 1
;; @004b                               v76 = iconst.i32 0
;; @004b                               store notrap aligned v76, v51  ; v76 = 0
;; @004b                               v77 = iconst.i32 0
;; @004b                               store notrap aligned v77, v51+4  ; v77 = 0
;; @004b                               v78 = iconst.i64 0
;; @004b                               store notrap aligned v78, v51+8  ; v78 = 0
;; @004b                               store notrap aligned v30, v29+40  ; v30 = 0
;; @004b                               v79 = iconst.i64 32
;; @004b                               v80 = ushr v68, v79  ; v79 = 32
;; @004b                               brif v80, block4, block3
;;
;;                                 block4:
;; @004b                               v81 = iconst.i64 0
;; @004b                               v82 = iadd.i64 v71, v81  ; v81 = 0
;; @004b                               v83 = iconst.i64 0
;; @004b                               v84 = iadd v82, v83  ; v83 = 0
;; @004b                               v85 = load.i64 notrap aligned v41+72
;; @004b                               store notrap aligned v85, v84+8
;; @004b                               v86 = iconst.i64 0
;; @004b                               v87 = iadd.i64 v29, v86  ; v86 = 0
;; @004b                               v88 = load.i64 notrap aligned v87
;; @004b                               store notrap aligned v88, v41+24
;; @004b                               v89 = load.i64 notrap aligned v87+8
;; @004b                               store notrap aligned v89, v41+72
;; @004b                               v90 = ireduce.i32 v68
;; @004b                               v91 = load.i64 notrap aligned v71+72
;; @004b                               v92 = uextend.i128 v71
;; @004b                               v93 = uextend.i128 v91
;; @004b                               v94 = iconst.i64 64
;; @004b                               v95 = uextend.i128 v94  ; v94 = 64
;; @004b                               v96 = ishl v93, v95
;; @004b                               v97 = bor v96, v92
;; @004b                               jump block5
;;
;;                                 block6 cold:
;; @004b                               trap user12
;;
;;                                 block5:
;; @004b                               br_table v90, block6, []
;;
;;                                 block3:
;; @004b                               v98 = iconst.i64 0
;; @004b                               v99 = iadd.i64 v29, v98  ; v98 = 0
;; @004b                               v100 = load.i64 notrap aligned v99
;; @004b                               store notrap aligned v100, v41+24
;; @004b                               v101 = load.i64 notrap aligned v99+8
;; @004b                               store notrap aligned v101, v41+72
;; @004b                               v102 = iconst.i64 0
;; @004b                               v103 = iadd.i64 v71, v102  ; v102 = 0
;; @004b                               v104 = iconst.i32 4
;; @004b                               v105 = iconst.i64 16
;; @004b                               v106 = iadd v103, v105  ; v105 = 16
;; @004b                               store notrap aligned v104, v106  ; v104 = 4
;; @004b                               v107 = iconst.i64 104
;; @004b                               v108 = iadd.i64 v71, v107  ; v107 = 104
;; @004b                               v109 = load.i64 notrap aligned v108+8
;; @004b                               v110 = iconst.i32 0
;; @004b                               store notrap aligned v110, v108  ; v110 = 0
;; @004b                               v111 = iconst.i32 0
;; @004b                               store notrap aligned v111, v108+4  ; v111 = 0
;; @004b                               v112 = iconst.i64 0
;; @004b                               store notrap aligned v112, v108+8  ; v112 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
