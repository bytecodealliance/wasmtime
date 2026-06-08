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
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003a                               v2 = iconst.i32 1
;; @003a                               v3 = call fn0(v0, v2)  ; v2 = 1
;; @003c                               trapz v3, user16
;; @003c                               v4 = iconst.i32 1
;; @003c                               v5 = iconst.i32 0
;; @003c                               v6 = call fn1(v0, v3, v4, v5)  ; v4 = 1, v5 = 0
;; @003c                               v7 = load.i64 notrap aligned v6+72
;; @003c                               v8 = uextend.i128 v6
;; @003c                               v9 = uextend.i128 v7
;; @003c                               v10 = iconst.i64 64
;; @003c                               v11 = uextend.i128 v10  ; v10 = 64
;; @003c                               v12 = ishl v9, v11
;; @003c                               v13 = bor v12, v8
;; @003e                               v14 = ireduce.i64 v13
;; @003e                               v15 = iconst.i64 64
;; @003e                               v16 = uextend.i128 v15  ; v15 = 64
;; @003e                               v17 = ushr v13, v16
;; @003e                               v18 = ireduce.i64 v17
;; @003e                               trapz v14, user16
;; @003e                               v19 = load.i64 notrap aligned v14+72
;; @003e                               v20 = icmp eq v19, v18
;; @003e                               trapz v20, user23
;; @003e                               v21 = iconst.i64 1
;; @003e                               v22 = iadd v19, v21  ; v21 = 1
;; @003e                               store notrap aligned v22, v14+72
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
;; @003e                               v41 = iconst.i32 8
;; @003e                               v42 = imul.i32 v39, v41  ; v41 = 8
;; @003e                               v43 = uextend.i64 v42
;; @003e                               v44 = iadd.i64 v36, v43
;; @003e                               v45 = load.i64 notrap aligned v44
;; @003e                               v46 = icmp eq v45, v24
;; @003e                               v47 = iconst.i32 1
;; @003e                               v48 = iadd.i32 v39, v47  ; v47 = 1
;; @003e                               brif v46, block6, block4(v48)
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned v29, v27+64
;; @003e                               v49 = iconst.i64 120
;; @003e                               v50 = iadd.i64 v27, v49  ; v49 = 120
;; @003e                               v51 = iconst.i64 0
;; @003e                               v52 = iadd.i64 v27, v51  ; v51 = 0
;; @003e                               v53 = iconst.i32 3
;; @003e                               v54 = iconst.i64 16
;; @003e                               v55 = iadd v52, v54  ; v54 = 16
;; @003e                               store notrap aligned v53, v55  ; v53 = 3
;; @003e                               v56 = iconst.i64 0
;; @003e                               v57 = iconst.i64 0
;; @003e                               store notrap aligned v56, v29+48  ; v56 = 0
;; @003e                               store notrap aligned v57, v29+56  ; v57 = 0
;; @003e                               v58 = load.i64 notrap aligned readonly v0+8
;; @003e                               v59 = iconst.i64 0
;; @003e                               v60 = iadd v52, v59  ; v59 = 0
;; @003e                               v61 = load.i64 notrap aligned v58+72
;; @003e                               store notrap aligned v61, v60+8
;; @003e                               v62 = load.i64 notrap aligned v27+72
;; @003e                               v63 = uextend.i128 v27
;; @003e                               v64 = uextend.i128 v62
;; @003e                               v65 = iconst.i64 64
;; @003e                               v66 = uextend.i128 v65  ; v65 = 64
;; @003e                               v67 = ishl v64, v66
;; @003e                               v68 = bor v67, v63
;; @003e                               v70 = iconst.i64 0
;; @003e                               v71 = iadd.i64 v14, v70  ; v70 = 0
;; @003e                               v72 = iconst.i64 16
;; @003e                               v73 = iadd v71, v72  ; v72 = 16
;; @003e                               v74 = load.i32 notrap aligned v73
;; @003e                               v75 = iconst.i32 0
;; @003e                               v76 = icmp ne v74, v75  ; v75 = 0
;; @003e                               brif v76, block9, block8
;;
;;                                 block8:
;; @003e                               v77 = iconst.i64 104
;; @003e                               v78 = iadd.i64 v14, v77  ; v77 = 104
;; @003e                               v79 = load.i64 notrap aligned v78+8
;; @003e                               v80 = load.i32 notrap aligned v78
;; @003e                               v81 = iconst.i32 1
;; @003e                               v82 = iadd v80, v81  ; v81 = 1
;; @003e                               store notrap aligned v82, v78
;; @003e                               v83 = uextend.i64 v80
;; @003e                               v84 = iconst.i64 16
;; @003e                               v85 = imul v83, v84  ; v84 = 16
;; @003e                               v86 = iadd v79, v85
;; @003e                               jump block10(v86)
;;
;;                                 block9:
;; @003e                               v87 = iconst.i64 120
;; @003e                               v88 = iadd.i64 v14, v87  ; v87 = 120
;; @003e                               v89 = load.i64 notrap aligned v88+8
;; @003e                               v90 = load.i32 notrap aligned v88
;; @003e                               v91 = iconst.i32 1
;; @003e                               v92 = iadd v90, v91  ; v91 = 1
;; @003e                               store notrap aligned v92, v88
;; @003e                               v93 = uextend.i64 v90
;; @003e                               v94 = iconst.i64 16
;; @003e                               v95 = imul v93, v94  ; v94 = 16
;; @003e                               v96 = iadd v89, v95
;; @003e                               jump block10(v96)
;;
;;                                 block10(v69: i64):
;; @003e                               store.i128 notrap aligned v68, v69
;; @003e                               v97 = iconst.i64 0
;; @003e                               v98 = iadd.i64 v14, v97  ; v97 = 0
;; @003e                               v99 = iconst.i32 1
;; @003e                               v100 = iconst.i64 16
;; @003e                               v101 = iadd v98, v100  ; v100 = 16
;; @003e                               store notrap aligned v99, v101  ; v99 = 1
;; @003e                               v102 = load.i64 notrap aligned v14+64
;; @003e                               store.i64 notrap aligned v32, v102+48
;; @003e                               store.i64 notrap aligned v33, v102+56
;; @003e                               v103 = iconst.i64 2
;; @003e                               v104 = load.i64 notrap aligned v0+8
;; @003e                               store notrap aligned v103, v104+88  ; v103 = 2
;; @003e                               store.i64 notrap aligned v14, v104+96
;; @003e                               v105 = iconst.i64 0
;; @003e                               v106 = iadd v98, v105  ; v105 = 0
;; @003e                               v107 = load.i64 notrap aligned v106
;; @003e                               store notrap aligned v107, v58+24
;; @003e                               v108 = load.i64 notrap aligned v106+8
;; @003e                               store notrap aligned v108, v58+72
;; @003e                               v109 = iconst.i64 80
;; @003e                               v110 = iadd.i64 v29, v109  ; v109 = 80
;; @003e                               v111 = load.i64 notrap aligned v110
;; @003e                               v112 = iconst.i64 -24
;; @003e                               v113 = iadd v111, v112  ; v112 = -24
;; @003e                               v114 = iconst.i64 80
;; @003e                               v115 = iadd v102, v114  ; v114 = 80
;; @003e                               v116 = load.i64 notrap aligned v115
;; @003e                               v117 = iconst.i64 -24
;; @003e                               v118 = iadd v116, v117  ; v117 = -24
;; @003e                               v119 = stack_addr.i64 ss0
;; @003e                               v120 = load.i64 notrap aligned v118
;; @003e                               store notrap aligned v120, v119
;; @003e                               v121 = load.i64 notrap aligned v113
;; @003e                               store notrap aligned v121, v118
;; @003e                               v122 = load.i64 notrap aligned v118+8
;; @003e                               store notrap aligned v122, v119+8
;; @003e                               v123 = load.i64 notrap aligned v113+8
;; @003e                               store notrap aligned v123, v118+8
;; @003e                               v124 = load.i64 notrap aligned v118+16
;; @003e                               store notrap aligned v124, v119+16
;; @003e                               v125 = load.i64 notrap aligned v113+16
;; @003e                               store notrap aligned v125, v118+16
;; @003e                               v126 = iconst.i64 3
;; @003e                               v127 = iconst.i64 32
;; @003e                               v128 = ishl v126, v127  ; v126 = 3, v127 = 32
;; @003e                               v129 = stack_switch v113, v119, v128
;; @003e                               v130 = iconst.i64 120
;; @003e                               v131 = iadd.i64 v27, v130  ; v130 = 120
;; @003e                               v132 = load.i64 notrap aligned v131+8
;; @003e                               v133 = iconst.i32 0
;; @003e                               store notrap aligned v133, v131  ; v133 = 0
;; @003e                               v134 = iconst.i32 0
;; @003e                               store notrap aligned v134, v131+4  ; v134 = 0
;; @003e                               v135 = iconst.i64 0
;; @003e                               store notrap aligned v135, v131+8  ; v135 = 0
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
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0047                               v2 = iconst.i32 0
;; @0047                               v3 = call fn0(v0, v2)  ; v2 = 0
;; @0049                               trapz v3, user16
;; @0049                               v4 = iconst.i32 0
;; @0049                               v5 = iconst.i32 0
;; @0049                               v6 = call fn1(v0, v3, v4, v5)  ; v4 = 0, v5 = 0
;; @0049                               v7 = load.i64 notrap aligned v6+72
;; @0049                               v8 = uextend.i128 v6
;; @0049                               v9 = uextend.i128 v7
;; @0049                               v10 = iconst.i64 64
;; @0049                               v11 = uextend.i128 v10  ; v10 = 64
;; @0049                               v12 = ishl v9, v11
;; @0049                               v13 = bor v12, v8
;; @004b                               jump block2
;;
;;                                 block2:
;; @004b                               v14 = ireduce.i64 v13
;; @004b                               v15 = iconst.i64 64
;; @004b                               v16 = uextend.i128 v15  ; v15 = 64
;; @004b                               v17 = ushr.i128 v13, v16
;; @004b                               v18 = ireduce.i64 v17
;; @004b                               trapz v14, user16
;; @004b                               v19 = load.i64 notrap aligned v14+72
;; @004b                               v20 = icmp eq v19, v18
;; @004b                               trapz v20, user23
;; @004b                               v21 = iconst.i64 1
;; @004b                               v22 = iadd v19, v21  ; v21 = 1
;; @004b                               store notrap aligned v22, v14+72
;; @004b                               v23 = load.i64 notrap aligned v14+64
;; @004b                               v24 = load.i64 notrap aligned v0+8
;; @004b                               v25 = load.i64 notrap aligned v24+88
;; @004b                               v26 = load.i64 notrap aligned v24+96
;; @004b                               store notrap aligned v25, v23+48
;; @004b                               store notrap aligned v26, v23+56
;; @004b                               v27 = iconst.i64 0
;; @004b                               store notrap aligned v27, v14+64  ; v27 = 0
;; @004b                               v28 = iconst.i64 2
;; @004b                               v29 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v28, v29+88  ; v28 = 2
;; @004b                               store notrap aligned v14, v29+96
;; @004b                               v30 = iconst.i64 0
;; @004b                               v31 = iadd v14, v30  ; v30 = 0
;; @004b                               v32 = iconst.i32 1
;; @004b                               v33 = iconst.i64 16
;; @004b                               v34 = iadd v31, v33  ; v33 = 16
;; @004b                               store notrap aligned v32, v34  ; v32 = 1
;; @004b                               v35 = iconst.i32 2
;; @004b                               v36 = iconst.i64 16
;; @004b                               v37 = iadd v26, v36  ; v36 = 16
;; @004b                               store notrap aligned v35, v37  ; v35 = 2
;; @004b                               v38 = load.i64 notrap aligned readonly v0+8
;; @004b                               v39 = iconst.i64 0
;; @004b                               v40 = iadd v26, v39  ; v39 = 0
;; @004b                               v41 = load.i64 notrap aligned v38+72
;; @004b                               store notrap aligned v41, v40+8
;; @004b                               v42 = load.i64 notrap aligned v38+24
;; @004b                               store notrap aligned v42, v40
;; @004b                               v43 = iconst.i64 0
;; @004b                               v44 = iadd v31, v43  ; v43 = 0
;; @004b                               v45 = load.i64 notrap aligned v44
;; @004b                               store notrap aligned v45, v38+24
;; @004b                               v46 = load.i64 notrap aligned v44+8
;; @004b                               store notrap aligned v46, v38+72
;; @004b                               v47 = iconst.i64 24
;; @004b                               v48 = iadd v26, v47  ; v47 = 24
;; @004b                               v49 = iconst.i32 1
;; @004b                               v50 = stack_addr.i64 ss0
;; @004b                               store notrap aligned v49, v48+4  ; v49 = 1
;; @004b                               store notrap aligned v50, v48+8
;; @004b                               v51 = iconst.i64 48
;; @004b                               v52 = iadd.i64 v0, v51  ; v51 = 48
;; @004b                               v53 = iconst.i32 1
;; @004b                               v54 = load.i64 notrap aligned v48+8
;; @004b                               store notrap aligned v52, v54
;; @004b                               store notrap aligned v53, v48  ; v53 = 1
;; @004b                               v55 = iconst.i32 0
;; @004b                               store notrap aligned v55, v26+40  ; v55 = 0
;; @004b                               v56 = iconst.i64 1
;; @004b                               v57 = iconst.i64 32
;; @004b                               v58 = ishl v56, v57  ; v56 = 1, v57 = 32
;; @004b                               v59 = iconst.i64 80
;; @004b                               v60 = iadd v23, v59  ; v59 = 80
;; @004b                               v61 = load.i64 notrap aligned v60
;; @004b                               v62 = iconst.i64 -24
;; @004b                               v63 = iadd v61, v62  ; v62 = -24
;; @004b                               v64 = stack_switch v63, v63, v58
;; @004b                               v65 = load.i64 notrap aligned v0+8
;; @004b                               v66 = load.i64 notrap aligned v65+88
;; @004b                               v67 = load.i64 notrap aligned v65+96
;; @004b                               v68 = load.i64 notrap aligned v0+8
;; @004b                               store notrap aligned v25, v68+88
;; @004b                               store notrap aligned v26, v68+96
;; @004b                               v69 = iconst.i32 1
;; @004b                               v70 = iconst.i64 16
;; @004b                               v71 = iadd v26, v70  ; v70 = 16
;; @004b                               store notrap aligned v69, v71  ; v69 = 1
;; @004b                               v72 = iconst.i32 0
;; @004b                               store notrap aligned v72, v48  ; v72 = 0
;; @004b                               v73 = iconst.i32 0
;; @004b                               store notrap aligned v73, v48+4  ; v73 = 0
;; @004b                               v74 = iconst.i64 0
;; @004b                               store notrap aligned v74, v48+8  ; v74 = 0
;; @004b                               store notrap aligned v27, v26+40  ; v27 = 0
;; @004b                               v75 = iconst.i64 32
;; @004b                               v76 = ushr v64, v75  ; v75 = 32
;; @004b                               brif v76, block4, block3
;;
;;                                 block4:
;; @004b                               v77 = iconst.i64 0
;; @004b                               v78 = iadd.i64 v67, v77  ; v77 = 0
;; @004b                               v79 = iconst.i64 0
;; @004b                               v80 = iadd v78, v79  ; v79 = 0
;; @004b                               v81 = load.i64 notrap aligned v38+72
;; @004b                               store notrap aligned v81, v80+8
;; @004b                               v82 = iconst.i64 0
;; @004b                               v83 = iadd.i64 v26, v82  ; v82 = 0
;; @004b                               v84 = load.i64 notrap aligned v83
;; @004b                               store notrap aligned v84, v38+24
;; @004b                               v85 = load.i64 notrap aligned v83+8
;; @004b                               store notrap aligned v85, v38+72
;; @004b                               v86 = ireduce.i32 v64
;; @004b                               v87 = load.i64 notrap aligned v67+72
;; @004b                               v88 = uextend.i128 v67
;; @004b                               v89 = uextend.i128 v87
;; @004b                               v90 = iconst.i64 64
;; @004b                               v91 = uextend.i128 v90  ; v90 = 64
;; @004b                               v92 = ishl v89, v91
;; @004b                               v93 = bor v92, v88
;; @004b                               jump block5
;;
;;                                 block6 cold:
;; @004b                               trap user12
;;
;;                                 block5:
;; @004b                               br_table v86, block6, []
;;
;;                                 block3:
;; @004b                               v94 = iconst.i64 0
;; @004b                               v95 = iadd.i64 v26, v94  ; v94 = 0
;; @004b                               v96 = load.i64 notrap aligned v95
;; @004b                               store notrap aligned v96, v38+24
;; @004b                               v97 = load.i64 notrap aligned v95+8
;; @004b                               store notrap aligned v97, v38+72
;; @004b                               v98 = iconst.i64 0
;; @004b                               v99 = iadd.i64 v67, v98  ; v98 = 0
;; @004b                               v100 = iconst.i32 4
;; @004b                               v101 = iconst.i64 16
;; @004b                               v102 = iadd v99, v101  ; v101 = 16
;; @004b                               store notrap aligned v100, v102  ; v100 = 4
;; @004b                               v103 = iconst.i64 104
;; @004b                               v104 = iadd.i64 v67, v103  ; v103 = 104
;; @004b                               v105 = load.i64 notrap aligned v104+8
;; @004b                               v106 = iconst.i32 0
;; @004b                               store notrap aligned v106, v104  ; v106 = 0
;; @004b                               v107 = iconst.i32 0
;; @004b                               store notrap aligned v107, v104+4  ; v107 = 0
;; @004b                               v108 = iconst.i64 0
;; @004b                               store notrap aligned v108, v104+8  ; v108 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
