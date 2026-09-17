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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 206 ""
;;     region3 = 106 ""
;;     region4 = 225 ""
;;     region5 = 214 ""
;;     region6 = 13 ""
;;     region7 = 82 ""
;;     region8 = 55 ""
;;     region9 = 153 ""
;;     region10 = 118 ""
;;     region11 = 255 ""
;;     region12 = 231 ""
;;     region13 = 243 ""
;;     region14 = 209 ""
;;     region15 = 23 ""
;;     region16 = 224 ""
;;     region17 = 179 ""
;;     region18 = 211 ""
;;     region19 = 187 ""
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
;; @003e                               v33 = load.i64 notrap aligned region4 v30+64
;; @003e                               v34 = load.i64 notrap aligned region4 v30+72
;; @003e                               v35 = iconst.i64 40
;; @003e                               v36 = iadd v34, v35  ; v35 = 40
;; @003e                               v37 = load.i64 notrap aligned region5 v36+8
;; @003e                               v38 = load.i32 notrap aligned region6 v34+56
;; @003e                               v39 = load.i32 notrap aligned region7 v36
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
;; @003e                               v46 = load.i64 notrap aligned region8 v45
;; @003e                               v47 = icmp eq v46, v25
;; @003e                               v48 = iconst.i32 1
;; @003e                               v49 = iadd.i32 v40, v48  ; v48 = 1
;; @003e                               brif v47, block6, block4(v49)
;;
;;                                 block7 cold:
;; @003e                               trap user22
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned region9 v30, v28+80
;; @003e                               v50 = iconst.i64 144
;; @003e                               v51 = iadd.i64 v28, v50  ; v50 = 144
;; @003e                               v52 = iconst.i32 1
;; @003e                               v53 = stack_addr.i64 ss0
;; @003e                               store notrap aligned region10 v52, v51+4  ; v52 = 1
;; @003e                               store notrap aligned region5 v53, v51+8
;; @003e                               v54 = iconst.i64 0
;; @003e                               v55 = iadd.i64 v28, v54  ; v54 = 0
;; @003e                               v56 = iconst.i32 3
;; @003e                               store notrap aligned region11 v56, v55+32  ; v56 = 3
;; @003e                               v57 = iconst.i64 0
;; @003e                               v58 = iconst.i64 0
;; @003e                               store notrap aligned region4 v57, v30+64  ; v57 = 0
;; @003e                               store notrap aligned region4 v58, v30+72  ; v58 = 0
;; @003e                               v59 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v60 = iconst.i64 0
;; @003e                               v61 = iadd v55, v60  ; v60 = 0
;; @003e                               v62 = load.i64 notrap aligned region12 v59+72
;; @003e                               v63 = load.i64 notrap aligned region13 v59+64
;; @003e                               v64 = load.i64 notrap aligned region14 v59+80
;; @003e                               store notrap aligned region15 v62, v61+8
;; @003e                               store notrap aligned region16 v63, v61+16
;; @003e                               store notrap aligned region6 v64, v61+24
;; @003e                               v65 = load.i64 notrap aligned region2 v28+88
;; @003e                               v66 = uextend.i128 v28
;; @003e                               v67 = uextend.i128 v65
;; @003e                               v68 = iconst.i64 64
;; @003e                               v69 = uextend.i128 v68  ; v68 = 64
;; @003e                               v70 = ishl v67, v69
;; @003e                               v71 = bor v70, v66
;; @003e                               v73 = iconst.i64 0
;; @003e                               v74 = iadd.i64 v15, v73  ; v73 = 0
;; @003e                               v75 = load.i32 notrap aligned region11 v74+32
;; @003e                               v76 = iconst.i32 0
;; @003e                               v77 = icmp ne v75, v76  ; v76 = 0
;; @003e                               brif v77, block9, block8
;;
;;                                 block8:
;; @003e                               v78 = iconst.i64 120
;; @003e                               v79 = iadd.i64 v15, v78  ; v78 = 120
;; @003e                               v80 = load.i64 notrap aligned region5 v79+8
;; @003e                               v81 = load.i32 notrap aligned region7 v79
;; @003e                               v82 = iconst.i32 1
;; @003e                               v83 = iadd v81, v82  ; v82 = 1
;; @003e                               store notrap aligned region7 v83, v79
;; @003e                               v84 = uextend.i64 v81
;; @003e                               v85 = iconst.i64 16
;; @003e                               v86 = imul v84, v85  ; v85 = 16
;; @003e                               v87 = iadd v80, v86
;; @003e                               jump block10(v87)
;;
;;                                 block9:
;; @003e                               v88 = iconst.i64 144
;; @003e                               v89 = iadd.i64 v15, v88  ; v88 = 144
;; @003e                               v90 = load.i64 notrap aligned region5 v89+8
;; @003e                               v91 = load.i32 notrap aligned region7 v89
;; @003e                               v92 = iconst.i32 1
;; @003e                               v93 = iadd v91, v92  ; v92 = 1
;; @003e                               store notrap aligned region7 v93, v89
;; @003e                               v94 = uextend.i64 v91
;; @003e                               v95 = iconst.i64 16
;; @003e                               v96 = imul v94, v95  ; v95 = 16
;; @003e                               v97 = iadd v90, v96
;; @003e                               jump block10(v97)
;;
;;                                 block10(v72: i64):
;; @003e                               store.i128 notrap aligned region8 v71, v72
;; @003e                               v98 = iconst.i64 0
;; @003e                               v99 = iadd.i64 v15, v98  ; v98 = 0
;; @003e                               v100 = iconst.i32 1
;; @003e                               store notrap aligned region11 v100, v99+32  ; v100 = 1
;; @003e                               v101 = load.i64 notrap aligned region9 v15+80
;; @003e                               store.i64 notrap aligned region4 v33, v101+64
;; @003e                               store.i64 notrap aligned region4 v34, v101+72
;; @003e                               v102 = iconst.i64 2
;; @003e                               v103 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               store notrap aligned region3 v102, v103+88  ; v102 = 2
;; @003e                               store.i64 notrap aligned region3 v15, v103+96
;; @003e                               v104 = iconst.i64 0
;; @003e                               v105 = iadd v99, v104  ; v104 = 0
;; @003e                               v106 = load.i64 notrap aligned region17 v105
;; @003e                               store notrap aligned region1 v106, v59+24
;; @003e                               v107 = load.i64 notrap aligned region15 v105+8
;; @003e                               store notrap aligned region12 v107, v59+72
;; @003e                               v108 = load.i64 notrap aligned region16 v105+16
;; @003e                               store notrap aligned region13 v108, v59+64
;; @003e                               v109 = load.i64 notrap aligned region6 v105+24
;; @003e                               store notrap aligned region14 v109, v59+80
;; @003e                               v110 = iconst.i64 96
;; @003e                               v111 = iadd.i64 v30, v110  ; v110 = 96
;; @003e                               v112 = load.i64 notrap aligned region18 v111
;; @003e                               v113 = iconst.i64 -24
;; @003e                               v114 = iadd v112, v113  ; v113 = -24
;; @003e                               v115 = iconst.i64 96
;; @003e                               v116 = iadd v101, v115  ; v115 = 96
;; @003e                               v117 = load.i64 notrap aligned region18 v116
;; @003e                               v118 = iconst.i64 -24
;; @003e                               v119 = iadd v117, v118  ; v118 = -24
;; @003e                               v120 = stack_addr.i64 ss1
;; @003e                               v121 = load.i64 notrap aligned region8 v119
;; @003e                               store notrap aligned region19 v121, v120
;; @003e                               v122 = load.i64 notrap aligned region8 v114
;; @003e                               store notrap aligned region8 v122, v119
;; @003e                               v123 = load.i64 notrap aligned region8 v119+8
;; @003e                               store notrap aligned region19 v123, v120+8
;; @003e                               v124 = load.i64 notrap aligned region8 v114+8
;; @003e                               store notrap aligned region8 v124, v119+8
;; @003e                               v125 = load.i64 notrap aligned region8 v119+16
;; @003e                               store notrap aligned region19 v125, v120+16
;; @003e                               v126 = load.i64 notrap aligned region8 v114+16
;; @003e                               store notrap aligned region8 v126, v119+16
;; @003e                               v127 = iconst.i64 3
;; @003e                               v128 = iconst.i64 32
;; @003e                               v129 = ishl v127, v128  ; v127 = 3, v128 = 32
;; @003e                               v130 = stack_switch v114, v120, v129
;; @003e                               v131 = iconst.i64 32
;; @003e                               v132 = ushr v130, v131  ; v131 = 32
;; @003e                               v133 = iconst.i64 5
;; @003e                               v134 = icmp eq v132, v133  ; v133 = 5
;; @003e                               brif v134, block11, block12
;;
;;                                 block11 cold:
;; @003e                               v135 = iconst.i64 144
;; @003e                               v136 = iadd.i64 v28, v135  ; v135 = 144
;; @003e                               v137 = load.i64 notrap aligned region5 v136+8
;; @003e                               v138 = load.i32 notrap aligned region8 v137
;; @003e                               v139 = iconst.i32 0
;; @003e                               store notrap aligned region7 v139, v136  ; v139 = 0
;; @003e                               v140 = iconst.i32 0
;; @003e                               store notrap aligned region10 v140, v136+4  ; v140 = 0
;; @003e                               v141 = iconst.i64 0
;; @003e                               store notrap aligned region5 v141, v136+8  ; v141 = 0
;; @003e                               try_call fn2(v0, v138), sig2, block13, [ context v0 ]
;;
;;                                 block13:
;; @003e                               trap user12
;;
;;                                 block12:
;; @003e                               v142 = iconst.i64 144
;; @003e                               v143 = iadd.i64 v28, v142  ; v142 = 144
;; @003e                               v144 = load.i64 notrap aligned region5 v143+8
;; @003e                               v145 = iconst.i32 0
;; @003e                               store notrap aligned region7 v145, v143  ; v145 = 0
;; @003e                               v146 = iconst.i32 0
;; @003e                               store notrap aligned region10 v146, v143+4  ; v146 = 0
;; @003e                               v147 = iconst.i64 0
;; @003e                               store notrap aligned region5 v147, v143+8  ; v147 = 0
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i128) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 206 ""
;;     region3 = 153 ""
;;     region4 = 106 ""
;;     region5 = 225 ""
;;     region6 = 255 ""
;;     region7 = 231 ""
;;     region8 = 243 ""
;;     region9 = 209 ""
;;     region10 = 23 ""
;;     region11 = 224 ""
;;     region12 = 13 ""
;;     region13 = 179 ""
;;     region14 = 118 ""
;;     region15 = 214 ""
;;     region16 = 55 ""
;;     region17 = 82 ""
;;     region18 = 211 ""
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
;; @004b                               v24 = load.i64 notrap aligned region3 v15+80
;; @004b                               v25 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v26 = load.i64 notrap aligned region4 v25+88
;; @004b                               v27 = load.i64 notrap aligned region4 v25+96
;; @004b                               store notrap aligned region5 v26, v24+64
;; @004b                               store notrap aligned region5 v27, v24+72
;; @004b                               v28 = iconst.i64 0
;; @004b                               store notrap aligned region3 v28, v15+80  ; v28 = 0
;; @004b                               v29 = iconst.i64 2
;; @004b                               v30 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region4 v29, v30+88  ; v29 = 2
;; @004b                               store notrap aligned region4 v15, v30+96
;; @004b                               v31 = iconst.i64 0
;; @004b                               v32 = iadd v15, v31  ; v31 = 0
;; @004b                               v33 = iconst.i32 1
;; @004b                               store notrap aligned region6 v33, v32+32  ; v33 = 1
;; @004b                               v34 = iconst.i32 2
;; @004b                               store notrap aligned region6 v34, v27+32  ; v34 = 2
;; @004b                               v35 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v36 = iconst.i64 0
;; @004b                               v37 = iadd v27, v36  ; v36 = 0
;; @004b                               v38 = load.i64 notrap aligned region7 v35+72
;; @004b                               v39 = load.i64 notrap aligned region8 v35+64
;; @004b                               v40 = load.i64 notrap aligned region9 v35+80
;; @004b                               store notrap aligned region10 v38, v37+8
;; @004b                               store notrap aligned region11 v39, v37+16
;; @004b                               store notrap aligned region12 v40, v37+24
;; @004b                               v41 = load.i64 notrap aligned region1 v35+24
;; @004b                               store notrap aligned region13 v41, v37
;; @004b                               v42 = iconst.i64 0
;; @004b                               v43 = iadd v32, v42  ; v42 = 0
;; @004b                               v44 = load.i64 notrap aligned region13 v43
;; @004b                               store notrap aligned region1 v44, v35+24
;; @004b                               v45 = load.i64 notrap aligned region10 v43+8
;; @004b                               store notrap aligned region7 v45, v35+72
;; @004b                               v46 = load.i64 notrap aligned region11 v43+16
;; @004b                               store notrap aligned region8 v46, v35+64
;; @004b                               v47 = load.i64 notrap aligned region12 v43+24
;; @004b                               store notrap aligned region9 v47, v35+80
;; @004b                               v48 = iconst.i64 40
;; @004b                               v49 = iadd v27, v48  ; v48 = 40
;; @004b                               v50 = iconst.i32 1
;; @004b                               v51 = stack_addr.i64 ss0
;; @004b                               store notrap aligned region14 v50, v49+4  ; v50 = 1
;; @004b                               store notrap aligned region15 v51, v49+8
;; @004b                               v52 = iconst.i64 48
;; @004b                               v53 = iadd.i64 v0, v52  ; v52 = 48
;; @004b                               v54 = iconst.i32 1
;; @004b                               v55 = load.i64 notrap aligned region15 v49+8
;; @004b                               store notrap aligned region16 v53, v55
;; @004b                               store notrap aligned region17 v54, v49  ; v54 = 1
;; @004b                               v56 = iconst.i32 0
;; @004b                               store notrap aligned region12 v56, v27+56  ; v56 = 0
;; @004b                               v57 = iconst.i64 1
;; @004b                               v58 = iconst.i64 32
;; @004b                               v59 = ishl v57, v58  ; v57 = 1, v58 = 32
;; @004b                               v60 = iconst.i64 96
;; @004b                               v61 = iadd v24, v60  ; v60 = 96
;; @004b                               v62 = load.i64 notrap aligned region18 v61
;; @004b                               v63 = iconst.i64 -24
;; @004b                               v64 = iadd v62, v63  ; v63 = -24
;; @004b                               v65 = stack_switch v64, v64, v59
;; @004b                               v66 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v67 = load.i64 notrap aligned region4 v66+88
;; @004b                               v68 = load.i64 notrap aligned region4 v66+96
;; @004b                               v69 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region4 v26, v69+88
;; @004b                               store notrap aligned region4 v27, v69+96
;; @004b                               v70 = iconst.i32 1
;; @004b                               store notrap aligned region6 v70, v27+32  ; v70 = 1
;; @004b                               v71 = iconst.i32 0
;; @004b                               store notrap aligned region17 v71, v49  ; v71 = 0
;; @004b                               v72 = iconst.i32 0
;; @004b                               store notrap aligned region14 v72, v49+4  ; v72 = 0
;; @004b                               v73 = iconst.i64 0
;; @004b                               store notrap aligned region15 v73, v49+8  ; v73 = 0
;; @004b                               store notrap aligned region12 v28, v27+56  ; v28 = 0
;; @004b                               brif v65, block6, block3
;;
;;                                 block6:
;; @004b                               v74 = iconst.i64 32
;; @004b                               v75 = ushr.i64 v65, v74  ; v74 = 32
;; @004b                               v76 = iconst.i64 4
;; @004b                               v77 = icmp eq v75, v76  ; v76 = 4
;; @004b                               brif v77, block5, block4
;;
;;                                 block5 cold:
;; @004b                               v78 = iconst.i64 0
;; @004b                               v79 = iadd.i64 v68, v78  ; v78 = 0
;; @004b                               v80 = iconst.i32 5
;; @004b                               store notrap aligned region6 v80, v79+32  ; v80 = 5
;; @004b                               v81 = iconst.i64 0
;; @004b                               v82 = iadd.i64 v27, v81  ; v81 = 0
;; @004b                               v83 = load.i64 notrap aligned region13 v82
;; @004b                               store notrap aligned region1 v83, v35+24
;; @004b                               v84 = load.i64 notrap aligned region10 v82+8
;; @004b                               store notrap aligned region7 v84, v35+72
;; @004b                               v85 = load.i64 notrap aligned region11 v82+16
;; @004b                               store notrap aligned region8 v85, v35+64
;; @004b                               v86 = load.i64 notrap aligned region12 v82+24
;; @004b                               store notrap aligned region9 v86, v35+80
;; @004b                               v87 = iconst.i64 120
;; @004b                               v88 = iadd.i64 v68, v87  ; v87 = 120
;; @004b                               v89 = iconst.i32 0
;; @004b                               store notrap aligned region17 v89, v88  ; v89 = 0
;; @004b                               v90 = iconst.i32 0
;; @004b                               store notrap aligned region14 v90, v88+4  ; v90 = 0
;; @004b                               v91 = iconst.i64 0
;; @004b                               store notrap aligned region15 v91, v88+8  ; v91 = 0
;; @004b                               v92 = iconst.i64 144
;; @004b                               v93 = iadd.i64 v68, v92  ; v92 = 144
;; @004b                               v94 = iconst.i32 0
;; @004b                               store notrap aligned region17 v94, v93  ; v94 = 0
;; @004b                               v95 = iconst.i32 0
;; @004b                               store notrap aligned region14 v95, v93+4  ; v95 = 0
;; @004b                               v96 = iconst.i64 0
;; @004b                               store notrap aligned region15 v96, v93+8  ; v96 = 0
;; @004b                               try_call fn2(v0), sig2, block8, [ context v0 ]
;;
;;                                 block8:
;; @004b                               trap user12
;;
;;                                 block4:
;; @004b                               v97 = iconst.i64 0
;; @004b                               v98 = iadd.i64 v68, v97  ; v97 = 0
;; @004b                               v99 = iconst.i64 0
;; @004b                               v100 = iadd v98, v99  ; v99 = 0
;; @004b                               v101 = load.i64 notrap aligned region7 v35+72
;; @004b                               v102 = load.i64 notrap aligned region8 v35+64
;; @004b                               v103 = load.i64 notrap aligned region9 v35+80
;; @004b                               store notrap aligned region10 v101, v100+8
;; @004b                               store notrap aligned region11 v102, v100+16
;; @004b                               store notrap aligned region12 v103, v100+24
;; @004b                               v104 = iconst.i64 0
;; @004b                               v105 = iadd.i64 v27, v104  ; v104 = 0
;; @004b                               v106 = load.i64 notrap aligned region13 v105
;; @004b                               store notrap aligned region1 v106, v35+24
;; @004b                               v107 = load.i64 notrap aligned region10 v105+8
;; @004b                               store notrap aligned region7 v107, v35+72
;; @004b                               v108 = load.i64 notrap aligned region11 v105+16
;; @004b                               store notrap aligned region8 v108, v35+64
;; @004b                               v109 = load.i64 notrap aligned region12 v105+24
;; @004b                               store notrap aligned region9 v109, v35+80
;; @004b                               v110 = ireduce.i32 v65
;; @004b                               v111 = load.i64 notrap aligned region2 v68+88
;; @004b                               v112 = uextend.i128 v68
;; @004b                               v113 = uextend.i128 v111
;; @004b                               v114 = iconst.i64 64
;; @004b                               v115 = uextend.i128 v114  ; v114 = 64
;; @004b                               v116 = ishl v113, v115
;; @004b                               v117 = bor v116, v112
;; @004b                               jump block7
;;
;;                                 block9 cold:
;; @004b                               trap user12
;;
;;                                 block7:
;; @004b                               br_table v110, block9, []
;;
;;                                 block3:
;; @004b                               v118 = iconst.i64 0
;; @004b                               v119 = iadd.i64 v27, v118  ; v118 = 0
;; @004b                               v120 = load.i64 notrap aligned region13 v119
;; @004b                               store notrap aligned region1 v120, v35+24
;; @004b                               v121 = load.i64 notrap aligned region10 v119+8
;; @004b                               store notrap aligned region7 v121, v35+72
;; @004b                               v122 = load.i64 notrap aligned region11 v119+16
;; @004b                               store notrap aligned region8 v122, v35+64
;; @004b                               v123 = load.i64 notrap aligned region12 v119+24
;; @004b                               store notrap aligned region9 v123, v35+80
;; @004b                               v124 = iconst.i64 0
;; @004b                               v125 = iadd.i64 v68, v124  ; v124 = 0
;; @004b                               v126 = iconst.i32 4
;; @004b                               store notrap aligned region6 v126, v125+32  ; v126 = 4
;; @004b                               v127 = iconst.i64 120
;; @004b                               v128 = iadd.i64 v68, v127  ; v127 = 120
;; @004b                               v129 = load.i64 notrap aligned region15 v128+8
;; @004b                               v130 = iconst.i32 0
;; @004b                               store notrap aligned region17 v130, v128  ; v130 = 0
;; @004b                               v131 = iconst.i32 0
;; @004b                               store notrap aligned region14 v131, v128+4  ; v131 = 0
;; @004b                               v132 = iconst.i64 0
;; @004b                               store notrap aligned region15 v132, v128+8  ; v132 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
