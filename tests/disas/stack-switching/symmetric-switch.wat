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
;;     region19 = 240 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
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
;; @003c                               v6 = call fn1(v0, v3, v4, v5)  ; v4 = 1, v5 = 0
;; @003c                               v7 = load.i64 notrap aligned region2 v6+88
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
;; @003e                               v19 = load.i64 notrap aligned region2 v14+88
;; @003e                               v20 = icmp eq v19, v18
;; @003e                               trapz v20, user23
;; @003e                               v21 = iconst.i64 1
;; @003e                               v22 = iadd v19, v21  ; v21 = 1
;; @003e                               store notrap aligned region2 v22, v14+88
;; @003e                               v23 = iconst.i64 48
;; @003e                               v24 = iadd v0, v23  ; v23 = 48
;; @003e                               v25 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v26 = load.i64 notrap aligned region3 v25+88
;; @003e                               v27 = load.i64 notrap aligned region3 v25+96
;; @003e                               jump block2(v26, v27)
;;
;;                                 block2(v28: i64, v29: i64):
;; @003e                               v30 = iconst.i64 1
;; @003e                               v31 = icmp eq v28, v30  ; v30 = 1
;; @003e                               brif v31, block7, block3
;;
;;                                 block3:
;; @003e                               v32 = load.i64 notrap aligned region4 v29+64
;; @003e                               v33 = load.i64 notrap aligned region4 v29+72
;; @003e                               v34 = iconst.i64 40
;; @003e                               v35 = iadd v33, v34  ; v34 = 40
;; @003e                               v36 = load.i64 notrap aligned region5 v35+8
;; @003e                               v37 = load.i32 notrap aligned region6 v33+56
;; @003e                               v38 = load.i32 notrap aligned region7 v35
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
;; @003e                               v45 = load.i64 notrap aligned region8 v44
;; @003e                               v46 = icmp eq v45, v24
;; @003e                               v47 = iconst.i32 1
;; @003e                               v48 = iadd.i32 v39, v47  ; v47 = 1
;; @003e                               brif v46, block6, block4(v48)
;;
;;                                 block7 cold:
;; @003e                               trap user22
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned region9 v29, v27+80
;; @003e                               v49 = iconst.i64 136
;; @003e                               v50 = iadd.i64 v27, v49  ; v49 = 136
;; @003e                               v51 = iconst.i32 1
;; @003e                               v52 = stack_addr.i64 ss0
;; @003e                               store notrap aligned region10 v51, v50+4  ; v51 = 1
;; @003e                               store notrap aligned region5 v52, v50+8
;; @003e                               v53 = iconst.i64 0
;; @003e                               v54 = iadd.i64 v27, v53  ; v53 = 0
;; @003e                               v55 = iconst.i32 3
;; @003e                               store notrap aligned region11 v55, v54+32  ; v55 = 3
;; @003e                               v56 = iconst.i64 0
;; @003e                               v57 = iconst.i64 0
;; @003e                               store notrap aligned region4 v56, v29+64  ; v56 = 0
;; @003e                               store notrap aligned region4 v57, v29+72  ; v57 = 0
;; @003e                               v58 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v59 = iconst.i64 0
;; @003e                               v60 = iadd v54, v59  ; v59 = 0
;; @003e                               v61 = load.i64 notrap aligned region12 v58+72
;; @003e                               v62 = load.i64 notrap aligned region13 v58+64
;; @003e                               v63 = load.i64 notrap aligned region14 v58+80
;; @003e                               store notrap aligned region15 v61, v60+8
;; @003e                               store notrap aligned region16 v62, v60+16
;; @003e                               store notrap aligned region6 v63, v60+24
;; @003e                               v64 = load.i64 notrap aligned region2 v27+88
;; @003e                               v65 = uextend.i128 v27
;; @003e                               v66 = uextend.i128 v64
;; @003e                               v67 = iconst.i64 64
;; @003e                               v68 = uextend.i128 v67  ; v67 = 64
;; @003e                               v69 = ishl v66, v68
;; @003e                               v70 = bor v69, v65
;; @003e                               v72 = iconst.i64 0
;; @003e                               v73 = iadd.i64 v14, v72  ; v72 = 0
;; @003e                               v74 = load.i32 notrap aligned region11 v73+32
;; @003e                               v75 = iconst.i32 0
;; @003e                               v76 = icmp ne v74, v75  ; v75 = 0
;; @003e                               brif v76, block9, block8
;;
;;                                 block8:
;; @003e                               v77 = iconst.i64 120
;; @003e                               v78 = iadd.i64 v14, v77  ; v77 = 120
;; @003e                               v79 = load.i64 notrap aligned region5 v78+8
;; @003e                               v80 = load.i32 notrap aligned region7 v78
;; @003e                               v81 = iconst.i32 1
;; @003e                               v82 = iadd v80, v81  ; v81 = 1
;; @003e                               store notrap aligned region7 v82, v78
;; @003e                               v83 = uextend.i64 v80
;; @003e                               v84 = iconst.i64 16
;; @003e                               v85 = imul v83, v84  ; v84 = 16
;; @003e                               v86 = iadd v79, v85
;; @003e                               jump block10(v86)
;;
;;                                 block9:
;; @003e                               v87 = iconst.i64 136
;; @003e                               v88 = iadd.i64 v14, v87  ; v87 = 136
;; @003e                               v89 = load.i64 notrap aligned region5 v88+8
;; @003e                               v90 = load.i32 notrap aligned region7 v88
;; @003e                               v91 = iconst.i32 1
;; @003e                               v92 = iadd v90, v91  ; v91 = 1
;; @003e                               store notrap aligned region7 v92, v88
;; @003e                               v93 = uextend.i64 v90
;; @003e                               v94 = iconst.i64 16
;; @003e                               v95 = imul v93, v94  ; v94 = 16
;; @003e                               v96 = iadd v89, v95
;; @003e                               jump block10(v96)
;;
;;                                 block10(v71: i64):
;; @003e                               store.i128 notrap aligned region8 v70, v71
;; @003e                               v97 = iconst.i64 0
;; @003e                               v98 = iadd.i64 v14, v97  ; v97 = 0
;; @003e                               v99 = iconst.i32 1
;; @003e                               store notrap aligned region11 v99, v98+32  ; v99 = 1
;; @003e                               v100 = load.i64 notrap aligned region9 v14+80
;; @003e                               store.i64 notrap aligned region4 v32, v100+64
;; @003e                               store.i64 notrap aligned region4 v33, v100+72
;; @003e                               v101 = iconst.i64 2
;; @003e                               v102 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               store notrap aligned region3 v101, v102+88  ; v101 = 2
;; @003e                               store.i64 notrap aligned region3 v14, v102+96
;; @003e                               v103 = iconst.i64 0
;; @003e                               v104 = iadd v98, v103  ; v103 = 0
;; @003e                               v105 = load.i64 notrap aligned region17 v104
;; @003e                               store notrap aligned region1 v105, v58+24
;; @003e                               v106 = load.i64 notrap aligned region15 v104+8
;; @003e                               store notrap aligned region12 v106, v58+72
;; @003e                               v107 = load.i64 notrap aligned region16 v104+16
;; @003e                               store notrap aligned region13 v107, v58+64
;; @003e                               v108 = load.i64 notrap aligned region6 v104+24
;; @003e                               store notrap aligned region14 v108, v58+80
;; @003e                               v109 = iconst.i64 96
;; @003e                               v110 = iadd.i64 v29, v109  ; v109 = 96
;; @003e                               v111 = load.i64 notrap aligned region18 v110
;; @003e                               v112 = iconst.i64 -24
;; @003e                               v113 = iadd v111, v112  ; v112 = -24
;; @003e                               v114 = iconst.i64 96
;; @003e                               v115 = iadd v100, v114  ; v114 = 96
;; @003e                               v116 = load.i64 notrap aligned region18 v115
;; @003e                               v117 = iconst.i64 -24
;; @003e                               v118 = iadd v116, v117  ; v117 = -24
;; @003e                               v119 = stack_addr.i64 ss1
;; @003e                               v120 = load.i64 notrap aligned region8 v118
;; @003e                               store notrap aligned region19 v120, v119
;; @003e                               v121 = load.i64 notrap aligned region8 v113
;; @003e                               store notrap aligned region8 v121, v118
;; @003e                               v122 = load.i64 notrap aligned region8 v118+8
;; @003e                               store notrap aligned region19 v122, v119+8
;; @003e                               v123 = load.i64 notrap aligned region8 v113+8
;; @003e                               store notrap aligned region8 v123, v118+8
;; @003e                               v124 = load.i64 notrap aligned region8 v118+16
;; @003e                               store notrap aligned region19 v124, v119+16
;; @003e                               v125 = load.i64 notrap aligned region8 v113+16
;; @003e                               store notrap aligned region8 v125, v118+16
;; @003e                               v126 = iconst.i64 3
;; @003e                               v127 = iconst.i64 32
;; @003e                               v128 = ishl v126, v127  ; v126 = 3, v127 = 32
;; @003e                               v129 = stack_switch v113, v119, v128
;; @003e                               v130 = iconst.i64 32
;; @003e                               v131 = ushr v129, v130  ; v130 = 32
;; @003e                               v132 = iconst.i64 5
;; @003e                               v133 = icmp eq v131, v132  ; v132 = 5
;; @003e                               brif v133, block11, block12
;;
;;                                 block11 cold:
;; @003e                               v134 = iconst.i64 136
;; @003e                               v135 = iadd.i64 v27, v134  ; v134 = 136
;; @003e                               v136 = load.i64 notrap aligned region5 v135+8
;; @003e                               v137 = load.i32 notrap aligned region8 v136
;; @003e                               v138 = iconst.i32 0
;; @003e                               store notrap aligned region7 v138, v135  ; v138 = 0
;; @003e                               v139 = iconst.i32 0
;; @003e                               store notrap aligned region10 v139, v135+4  ; v139 = 0
;; @003e                               v140 = iconst.i64 0
;; @003e                               store notrap aligned region5 v140, v135+8  ; v140 = 0
;; @003e                               try_call fn2(v0, v137), sig2, block13, [ context v0 ]
;;
;;                                 block13:
;; @003e                               trap user12
;;
;;                                 block12:
;; @003e                               v141 = iconst.i64 136
;; @003e                               v142 = iadd.i64 v27, v141  ; v141 = 136
;; @003e                               v143 = load.i64 notrap aligned region5 v142+8
;; @003e                               v144 = iconst.i32 0
;; @003e                               store notrap aligned region7 v144, v142  ; v144 = 0
;; @003e                               v145 = iconst.i32 0
;; @003e                               store notrap aligned region10 v145, v142+4  ; v145 = 0
;; @003e                               v146 = iconst.i64 0
;; @003e                               store notrap aligned region5 v146, v142+8  ; v146 = 0
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
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
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
;; @0049                               v6 = call fn1(v0, v3, v4, v5)  ; v4 = 0, v5 = 0
;; @0049                               v7 = load.i64 notrap aligned region2 v6+88
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
;; @004b                               v19 = load.i64 notrap aligned region2 v14+88
;; @004b                               v20 = icmp eq v19, v18
;; @004b                               trapz v20, user23
;; @004b                               v21 = iconst.i64 1
;; @004b                               v22 = iadd v19, v21  ; v21 = 1
;; @004b                               store notrap aligned region2 v22, v14+88
;; @004b                               v23 = load.i64 notrap aligned region3 v14+80
;; @004b                               v24 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v25 = load.i64 notrap aligned region4 v24+88
;; @004b                               v26 = load.i64 notrap aligned region4 v24+96
;; @004b                               store notrap aligned region5 v25, v23+64
;; @004b                               store notrap aligned region5 v26, v23+72
;; @004b                               v27 = iconst.i64 0
;; @004b                               store notrap aligned region3 v27, v14+80  ; v27 = 0
;; @004b                               v28 = iconst.i64 2
;; @004b                               v29 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region4 v28, v29+88  ; v28 = 2
;; @004b                               store notrap aligned region4 v14, v29+96
;; @004b                               v30 = iconst.i64 0
;; @004b                               v31 = iadd v14, v30  ; v30 = 0
;; @004b                               v32 = iconst.i32 1
;; @004b                               store notrap aligned region6 v32, v31+32  ; v32 = 1
;; @004b                               v33 = iconst.i32 2
;; @004b                               store notrap aligned region6 v33, v26+32  ; v33 = 2
;; @004b                               v34 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v35 = iconst.i64 0
;; @004b                               v36 = iadd v26, v35  ; v35 = 0
;; @004b                               v37 = load.i64 notrap aligned region7 v34+72
;; @004b                               v38 = load.i64 notrap aligned region8 v34+64
;; @004b                               v39 = load.i64 notrap aligned region9 v34+80
;; @004b                               store notrap aligned region10 v37, v36+8
;; @004b                               store notrap aligned region11 v38, v36+16
;; @004b                               store notrap aligned region12 v39, v36+24
;; @004b                               v40 = load.i64 notrap aligned region1 v34+24
;; @004b                               store notrap aligned region13 v40, v36
;; @004b                               v41 = iconst.i64 0
;; @004b                               v42 = iadd v31, v41  ; v41 = 0
;; @004b                               v43 = load.i64 notrap aligned region13 v42
;; @004b                               store notrap aligned region1 v43, v34+24
;; @004b                               v44 = load.i64 notrap aligned region10 v42+8
;; @004b                               store notrap aligned region7 v44, v34+72
;; @004b                               v45 = load.i64 notrap aligned region11 v42+16
;; @004b                               store notrap aligned region8 v45, v34+64
;; @004b                               v46 = load.i64 notrap aligned region12 v42+24
;; @004b                               store notrap aligned region9 v46, v34+80
;; @004b                               v47 = iconst.i64 40
;; @004b                               v48 = iadd v26, v47  ; v47 = 40
;; @004b                               v49 = iconst.i32 1
;; @004b                               v50 = stack_addr.i64 ss0
;; @004b                               store notrap aligned region14 v49, v48+4  ; v49 = 1
;; @004b                               store notrap aligned region15 v50, v48+8
;; @004b                               v51 = iconst.i64 48
;; @004b                               v52 = iadd.i64 v0, v51  ; v51 = 48
;; @004b                               v53 = iconst.i32 1
;; @004b                               v54 = load.i64 notrap aligned region15 v48+8
;; @004b                               store notrap aligned region16 v52, v54
;; @004b                               store notrap aligned region17 v53, v48  ; v53 = 1
;; @004b                               v55 = iconst.i32 0
;; @004b                               store notrap aligned region12 v55, v26+56  ; v55 = 0
;; @004b                               v56 = iconst.i64 1
;; @004b                               v57 = iconst.i64 32
;; @004b                               v58 = ishl v56, v57  ; v56 = 1, v57 = 32
;; @004b                               v59 = iconst.i64 96
;; @004b                               v60 = iadd v23, v59  ; v59 = 96
;; @004b                               v61 = load.i64 notrap aligned region18 v60
;; @004b                               v62 = iconst.i64 -24
;; @004b                               v63 = iadd v61, v62  ; v62 = -24
;; @004b                               v64 = stack_switch v63, v63, v58
;; @004b                               v65 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v66 = load.i64 notrap aligned region4 v65+88
;; @004b                               v67 = load.i64 notrap aligned region4 v65+96
;; @004b                               v68 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region4 v25, v68+88
;; @004b                               store notrap aligned region4 v26, v68+96
;; @004b                               v69 = iconst.i32 1
;; @004b                               store notrap aligned region6 v69, v26+32  ; v69 = 1
;; @004b                               v70 = iconst.i32 0
;; @004b                               store notrap aligned region17 v70, v48  ; v70 = 0
;; @004b                               v71 = iconst.i32 0
;; @004b                               store notrap aligned region14 v71, v48+4  ; v71 = 0
;; @004b                               v72 = iconst.i64 0
;; @004b                               store notrap aligned region15 v72, v48+8  ; v72 = 0
;; @004b                               store notrap aligned region12 v27, v26+56  ; v27 = 0
;; @004b                               brif v64, block6, block3
;;
;;                                 block6:
;; @004b                               v73 = iconst.i64 32
;; @004b                               v74 = ushr.i64 v64, v73  ; v73 = 32
;; @004b                               v75 = iconst.i64 4
;; @004b                               v76 = icmp eq v74, v75  ; v75 = 4
;; @004b                               brif v76, block5, block4
;;
;;                                 block5 cold:
;; @004b                               v77 = iconst.i64 0
;; @004b                               v78 = iadd.i64 v67, v77  ; v77 = 0
;; @004b                               v79 = iconst.i32 5
;; @004b                               store notrap aligned region6 v79, v78+32  ; v79 = 5
;; @004b                               v80 = iconst.i64 0
;; @004b                               v81 = iadd.i64 v26, v80  ; v80 = 0
;; @004b                               v82 = load.i64 notrap aligned region13 v81
;; @004b                               store notrap aligned region1 v82, v34+24
;; @004b                               v83 = load.i64 notrap aligned region10 v81+8
;; @004b                               store notrap aligned region7 v83, v34+72
;; @004b                               v84 = load.i64 notrap aligned region11 v81+16
;; @004b                               store notrap aligned region8 v84, v34+64
;; @004b                               v85 = load.i64 notrap aligned region12 v81+24
;; @004b                               store notrap aligned region9 v85, v34+80
;; @004b                               v86 = iconst.i64 120
;; @004b                               v87 = iadd.i64 v67, v86  ; v86 = 120
;; @004b                               v88 = iconst.i32 0
;; @004b                               store notrap aligned region17 v88, v87  ; v88 = 0
;; @004b                               v89 = iconst.i32 0
;; @004b                               store notrap aligned region14 v89, v87+4  ; v89 = 0
;; @004b                               v90 = iconst.i64 0
;; @004b                               store notrap aligned region15 v90, v87+8  ; v90 = 0
;; @004b                               v91 = iconst.i64 136
;; @004b                               v92 = iadd.i64 v67, v91  ; v91 = 136
;; @004b                               v93 = iconst.i32 0
;; @004b                               store notrap aligned region17 v93, v92  ; v93 = 0
;; @004b                               v94 = iconst.i32 0
;; @004b                               store notrap aligned region14 v94, v92+4  ; v94 = 0
;; @004b                               v95 = iconst.i64 0
;; @004b                               store notrap aligned region15 v95, v92+8  ; v95 = 0
;; @004b                               try_call fn2(v0), sig2, block8, [ context v0 ]
;;
;;                                 block8:
;; @004b                               trap user12
;;
;;                                 block4:
;; @004b                               v96 = iconst.i64 0
;; @004b                               v97 = iadd.i64 v67, v96  ; v96 = 0
;; @004b                               v98 = iconst.i64 0
;; @004b                               v99 = iadd v97, v98  ; v98 = 0
;; @004b                               v100 = load.i64 notrap aligned region7 v34+72
;; @004b                               v101 = load.i64 notrap aligned region8 v34+64
;; @004b                               v102 = load.i64 notrap aligned region9 v34+80
;; @004b                               store notrap aligned region10 v100, v99+8
;; @004b                               store notrap aligned region11 v101, v99+16
;; @004b                               store notrap aligned region12 v102, v99+24
;; @004b                               v103 = iconst.i64 0
;; @004b                               v104 = iadd.i64 v26, v103  ; v103 = 0
;; @004b                               v105 = load.i64 notrap aligned region13 v104
;; @004b                               store notrap aligned region1 v105, v34+24
;; @004b                               v106 = load.i64 notrap aligned region10 v104+8
;; @004b                               store notrap aligned region7 v106, v34+72
;; @004b                               v107 = load.i64 notrap aligned region11 v104+16
;; @004b                               store notrap aligned region8 v107, v34+64
;; @004b                               v108 = load.i64 notrap aligned region12 v104+24
;; @004b                               store notrap aligned region9 v108, v34+80
;; @004b                               v109 = ireduce.i32 v64
;; @004b                               v110 = load.i64 notrap aligned region2 v67+88
;; @004b                               v111 = uextend.i128 v67
;; @004b                               v112 = uextend.i128 v110
;; @004b                               v113 = iconst.i64 64
;; @004b                               v114 = uextend.i128 v113  ; v113 = 64
;; @004b                               v115 = ishl v112, v114
;; @004b                               v116 = bor v115, v111
;; @004b                               jump block7
;;
;;                                 block9 cold:
;; @004b                               trap user12
;;
;;                                 block7:
;; @004b                               br_table v109, block9, []
;;
;;                                 block3:
;; @004b                               v117 = iconst.i64 0
;; @004b                               v118 = iadd.i64 v26, v117  ; v117 = 0
;; @004b                               v119 = load.i64 notrap aligned region13 v118
;; @004b                               store notrap aligned region1 v119, v34+24
;; @004b                               v120 = load.i64 notrap aligned region10 v118+8
;; @004b                               store notrap aligned region7 v120, v34+72
;; @004b                               v121 = load.i64 notrap aligned region11 v118+16
;; @004b                               store notrap aligned region8 v121, v34+64
;; @004b                               v122 = load.i64 notrap aligned region12 v118+24
;; @004b                               store notrap aligned region9 v122, v34+80
;; @004b                               v123 = iconst.i64 0
;; @004b                               v124 = iadd.i64 v67, v123  ; v123 = 0
;; @004b                               v125 = iconst.i32 4
;; @004b                               store notrap aligned region6 v125, v124+32  ; v125 = 4
;; @004b                               v126 = iconst.i64 120
;; @004b                               v127 = iadd.i64 v67, v126  ; v126 = 120
;; @004b                               v128 = load.i64 notrap aligned region15 v127+8
;; @004b                               v129 = iconst.i32 0
;; @004b                               store notrap aligned region17 v129, v127  ; v129 = 0
;; @004b                               v130 = iconst.i32 0
;; @004b                               store notrap aligned region14 v130, v127+4  ; v130 = 0
;; @004b                               v131 = iconst.i64 0
;; @004b                               store notrap aligned region15 v131, v127+8  ; v131 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
