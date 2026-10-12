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
;;     ss0 = explicit_slot 16, align = 16
;;     ss1 = explicit_slot 24, align = 8
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 130 ""
;;     region3 = 116 ""
;;     region4 = 44 ""
;;     region5 = 120 ""
;;     region6 = 100 ""
;;     region7 = 191 ""
;;     region8 = 106 ""
;;     region9 = 125 ""
;;     region10 = 139 ""
;;     region11 = 234 ""
;;     region12 = 82 ""
;;     region13 = 192 ""
;;     region14 = 145 ""
;;     region15 = 144 ""
;;     region16 = 231 ""
;;     region17 = 243 ""
;;     region18 = 209 ""
;;     region19 = 176 ""
;;     region20 = 62 ""
;;     region21 = 198 ""
;;     region22 = 124 ""
;;     region23 = 187 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i128) tail
;;     sig1 = (i64 vmctx, i64, i64, i64) -> i8 system_v
;;     sig2 = (i64 vmctx, i64, i32, i32, i32) -> i64 tail
;;     sig3 = (i64 vmctx, i32) -> i8 tail
;;     fn0 = colocated u0:1 sig0
;;     fn1 = colocated u268435456:1 sig1
;;     fn2 = colocated u805306368:41 sig2
;;     fn3 = colocated u805306368:43 sig3
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003a                               v2 = func_addr.i64 fn0
;; @003a                               v3 = func_addr.i64 fn1
;; @003a                               v4 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @003a                               v5 = load.i32 notrap aligned readonly can_move region3 v4+8
;; @003a                               store notrap aligned region4 v3, v0+120
;; @003a                               store notrap aligned region5 v2, v0+128
;; @003a                               store notrap aligned region6 v0, v0+144
;; @003a                               store notrap aligned region1 v5, v0+136
;; @003a                               v6 = iconst.i64 120
;; @003a                               v7 = iadd v0, v6  ; v6 = 120
;; @003c                               trapz v7, user16
;; @003c                               v8 = iconst.i32 1
;; @003c                               v9 = iconst.i32 0
;; @003c                               v10 = iconst.i32 0
;; @003c                               v11 = call fn2(v0, v7, v8, v9, v10)  ; v8 = 1, v9 = 0, v10 = 0
;; @003c                               v12 = load.i64 notrap aligned region7 v11+104
;; @003c                               v13 = uextend.i128 v11
;; @003c                               v14 = uextend.i128 v12
;; @003c                               v15 = iconst.i64 64
;; @003c                               v16 = uextend.i128 v15  ; v15 = 64
;; @003c                               v17 = ishl v14, v16
;; @003c                               v18 = bor v17, v13
;; @003e                               v19 = ireduce.i64 v18
;; @003e                               v20 = iconst.i64 64
;; @003e                               v21 = uextend.i128 v20  ; v20 = 64
;; @003e                               v22 = ushr v18, v21
;; @003e                               v23 = ireduce.i64 v22
;; @003e                               trapz v19, user16
;; @003e                               v24 = load.i64 notrap aligned region7 v19+104
;; @003e                               v25 = icmp eq v24, v23
;; @003e                               trapz v25, user23
;; @003e                               v26 = iconst.i64 1
;; @003e                               v27 = iadd v24, v26  ; v26 = 1
;; @003e                               store notrap aligned region7 v27, v19+104
;; @003e                               v28 = iconst.i64 48
;; @003e                               v29 = iadd v0, v28  ; v28 = 48
;; @003e                               v30 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v31 = load.i64 notrap aligned region8 v30+88
;; @003e                               v32 = load.i64 notrap aligned region8 v30+96
;; @003e                               jump block2(v31, v32)
;;
;;                                 block2(v33: i64, v34: i64):
;; @003e                               v35 = iconst.i64 1
;; @003e                               v36 = icmp eq v33, v35  ; v35 = 1
;; @003e                               brif v36, block7, block3
;;
;;                                 block3:
;; @003e                               v37 = load.i64 notrap aligned region9 v34+88
;; @003e                               v38 = load.i64 notrap aligned region9 v34+96
;; @003e                               v39 = iconst.i64 40
;; @003e                               v40 = iadd v38, v39  ; v39 = 40
;; @003e                               v41 = load.i64 notrap aligned region10 v40+8
;; @003e                               v42 = load.i32 notrap aligned region11 v38+56
;; @003e                               v43 = load.i32 notrap aligned region10 v40
;; @003e                               jump block4(v42)
;;
;;                                 block4(v44: i32):
;; @003e                               v45 = icmp ult v44, v43
;; @003e                               brif v45, block5, block2(v37, v38)
;;
;;                                 block5:
;; @003e                               v46 = iconst.i32 8
;; @003e                               v47 = imul.i32 v44, v46  ; v46 = 8
;; @003e                               v48 = uextend.i64 v47
;; @003e                               v49 = iadd.i64 v41, v48
;; @003e                               v50 = load.i64 notrap aligned region12 v49
;; @003e                               v51 = icmp eq v50, v29
;; @003e                               v52 = iconst.i32 1
;; @003e                               v53 = iadd.i32 v44, v52  ; v52 = 1
;; @003e                               brif v51, block6, block4(v53)
;;
;;                                 block7 cold:
;; @003e                               trap user22
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned region13 v34, v32+112
;; @003e                               v54 = iconst.i64 168
;; @003e                               v55 = iadd.i64 v32, v54  ; v54 = 168
;; @003e                               v56 = iconst.i32 1
;; @003e                               v57 = stack_addr.i64 ss0
;; @003e                               store notrap aligned region14 v56, v55+4  ; v56 = 1
;; @003e                               store notrap aligned region10 v57, v55+8
;; @003e                               v58 = iconst.i64 0
;; @003e                               v59 = iadd.i64 v32, v58  ; v58 = 0
;; @003e                               v60 = iconst.i32 3
;; @003e                               store notrap aligned region15 v60, v59+32  ; v60 = 3
;; @003e                               v61 = iconst.i64 0
;; @003e                               v62 = iconst.i64 0
;; @003e                               store notrap aligned region9 v61, v34+88  ; v61 = 0
;; @003e                               store notrap aligned region9 v62, v34+96  ; v62 = 0
;; @003e                               v63 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v64 = iconst.i64 0
;; @003e                               v65 = iadd v59, v64  ; v64 = 0
;; @003e                               v66 = load.i64 notrap aligned region16 v63+72
;; @003e                               v67 = load.i64 notrap aligned region17 v63+64
;; @003e                               v68 = load.i64 notrap aligned region18 v63+80
;; @003e                               store notrap aligned region6 v66, v65+8
;; @003e                               store notrap aligned region19 v67, v65+16
;; @003e                               store notrap aligned region20 v68, v65+24
;; @003e                               v69 = load.i64 notrap aligned region7 v32+104
;; @003e                               v70 = uextend.i128 v32
;; @003e                               v71 = uextend.i128 v69
;; @003e                               v72 = iconst.i64 64
;; @003e                               v73 = uextend.i128 v72  ; v72 = 64
;; @003e                               v74 = ishl v71, v73
;; @003e                               v75 = bor v74, v70
;; @003e                               v77 = iconst.i64 0
;; @003e                               v78 = iadd.i64 v19, v77  ; v77 = 0
;; @003e                               v79 = load.i32 notrap aligned region15 v78+32
;; @003e                               v80 = iconst.i32 0
;; @003e                               v81 = icmp ne v79, v80  ; v80 = 0
;; @003e                               brif v81, block9, block8
;;
;;                                 block8:
;; @003e                               v82 = iconst.i64 144
;; @003e                               v83 = iadd.i64 v19, v82  ; v82 = 144
;; @003e                               v84 = load.i64 notrap aligned region10 v83+8
;; @003e                               v85 = load.i32 notrap aligned region10 v83
;; @003e                               v86 = load.i32 notrap aligned region14 v83+4
;; @003e                               v87 = uextend.i64 v85
;; @003e                               v88 = uextend.i64 v86
;; @003e                               v89 = iconst.i64 1
;; @003e                               v90 = iadd v87, v89  ; v89 = 1
;; @003e                               v91 = icmp ule v90, v88
;; @003e                               trapz v91, user1
;; @003e                               v92 = ireduce.i32 v90
;; @003e                               store notrap aligned region10 v92, v83
;; @003e                               v93 = iconst.i64 16
;; @003e                               v94 = imul v87, v93  ; v93 = 16
;; @003e                               v95 = iadd v84, v94
;; @003e                               jump block10(v95)
;;
;;                                 block9:
;; @003e                               v96 = iconst.i64 168
;; @003e                               v97 = iadd.i64 v19, v96  ; v96 = 168
;; @003e                               v98 = load.i64 notrap aligned region10 v97+8
;; @003e                               v99 = load.i32 notrap aligned region10 v97
;; @003e                               v100 = load.i32 notrap aligned region14 v97+4
;; @003e                               v101 = uextend.i64 v99
;; @003e                               v102 = uextend.i64 v100
;; @003e                               v103 = iconst.i64 1
;; @003e                               v104 = iadd v101, v103  ; v103 = 1
;; @003e                               v105 = icmp ule v104, v102
;; @003e                               trapz v105, user1
;; @003e                               v106 = ireduce.i32 v104
;; @003e                               store notrap aligned region10 v106, v97
;; @003e                               v107 = iconst.i64 16
;; @003e                               v108 = imul v101, v107  ; v107 = 16
;; @003e                               v109 = iadd v98, v108
;; @003e                               jump block10(v109)
;;
;;                                 block10(v76: i64):
;; @003e                               store.i128 notrap aligned region12 v75, v76
;; @003e                               v110 = iconst.i64 0
;; @003e                               v111 = iadd.i64 v19, v110  ; v110 = 0
;; @003e                               v112 = iconst.i32 1
;; @003e                               store notrap aligned region15 v112, v111+32  ; v112 = 1
;; @003e                               v113 = load.i64 notrap aligned region13 v19+112
;; @003e                               store.i64 notrap aligned region9 v37, v113+88
;; @003e                               store.i64 notrap aligned region9 v38, v113+96
;; @003e                               v114 = iconst.i64 2
;; @003e                               v115 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               store notrap aligned region8 v114, v115+88  ; v114 = 2
;; @003e                               store.i64 notrap aligned region8 v19, v115+96
;; @003e                               v116 = iconst.i64 0
;; @003e                               v117 = iadd v111, v116  ; v116 = 0
;; @003e                               v118 = load.i64 notrap aligned region21 v117
;; @003e                               store notrap aligned region1 v118, v63+24
;; @003e                               v119 = load.i64 notrap aligned region6 v117+8
;; @003e                               store notrap aligned region16 v119, v63+72
;; @003e                               v120 = load.i64 notrap aligned region19 v117+16
;; @003e                               store notrap aligned region17 v120, v63+64
;; @003e                               v121 = load.i64 notrap aligned region20 v117+24
;; @003e                               store notrap aligned region18 v121, v63+80
;; @003e                               v122 = iconst.i64 120
;; @003e                               v123 = iadd.i64 v34, v122  ; v122 = 120
;; @003e                               v124 = load.i64 notrap aligned region22 v123
;; @003e                               v125 = iconst.i64 -24
;; @003e                               v126 = iadd v124, v125  ; v125 = -24
;; @003e                               v127 = iconst.i64 120
;; @003e                               v128 = iadd v113, v127  ; v127 = 120
;; @003e                               v129 = load.i64 notrap aligned region22 v128
;; @003e                               v130 = iconst.i64 -24
;; @003e                               v131 = iadd v129, v130  ; v130 = -24
;; @003e                               v132 = stack_addr.i64 ss1
;; @003e                               v133 = load.i64 notrap aligned region12 v131
;; @003e                               store notrap aligned region23 v133, v132
;; @003e                               v134 = load.i64 notrap aligned region12 v126
;; @003e                               store notrap aligned region12 v134, v131
;; @003e                               v135 = load.i64 notrap aligned region12 v131+8
;; @003e                               store notrap aligned region23 v135, v132+8
;; @003e                               v136 = load.i64 notrap aligned region12 v126+8
;; @003e                               store notrap aligned region12 v136, v131+8
;; @003e                               v137 = load.i64 notrap aligned region12 v131+16
;; @003e                               store notrap aligned region23 v137, v132+16
;; @003e                               v138 = load.i64 notrap aligned region12 v126+16
;; @003e                               store notrap aligned region12 v138, v131+16
;; @003e                               v139 = iconst.i64 3
;; @003e                               v140 = iconst.i64 32
;; @003e                               v141 = ishl v139, v140  ; v139 = 3, v140 = 32
;; @003e                               v142 = stack_switch v126, v132, v141
;; @003e                               v143 = iconst.i64 32
;; @003e                               v144 = ushr v142, v143  ; v143 = 32
;; @003e                               v145 = iconst.i64 5
;; @003e                               v146 = icmp eq v144, v145  ; v145 = 5
;; @003e                               brif v146, block11, block12
;;
;;                                 block11 cold:
;; @003e                               v147 = iconst.i64 168
;; @003e                               v148 = iadd.i64 v32, v147  ; v147 = 168
;; @003e                               v149 = load.i64 notrap aligned region10 v148+8
;; @003e                               v150 = load.i32 notrap aligned region12 v149
;; @003e                               v151 = iconst.i32 0
;; @003e                               store notrap aligned region10 v151, v148  ; v151 = 0
;; @003e                               v152 = iconst.i32 0
;; @003e                               store notrap aligned region14 v152, v148+4  ; v152 = 0
;; @003e                               v153 = iconst.i64 0
;; @003e                               store notrap aligned region10 v153, v148+8  ; v153 = 0
;; @003e                               try_call fn3(v0, v150), sig3, block13, [ context v0 ]
;;
;;                                 block13:
;; @003e                               trap user12
;;
;;                                 block12:
;; @003e                               v154 = iconst.i64 168
;; @003e                               v155 = iadd.i64 v32, v154  ; v154 = 168
;; @003e                               v156 = load.i64 notrap aligned region10 v155+8
;; @003e                               v157 = iconst.i32 0
;; @003e                               store notrap aligned region10 v157, v155  ; v157 = 0
;; @003e                               v158 = iconst.i32 0
;; @003e                               store notrap aligned region14 v158, v155+4  ; v158 = 0
;; @003e                               v159 = iconst.i64 0
;; @003e                               store notrap aligned region10 v159, v155+8  ; v159 = 0
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
;;     ss0 = explicit_slot 8, align = 8
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 130 ""
;;     region3 = 134 ""
;;     region4 = 44 ""
;;     region5 = 120 ""
;;     region6 = 100 ""
;;     region7 = 191 ""
;;     region8 = 192 ""
;;     region9 = 106 ""
;;     region10 = 125 ""
;;     region11 = 144 ""
;;     region12 = 231 ""
;;     region13 = 243 ""
;;     region14 = 209 ""
;;     region15 = 176 ""
;;     region16 = 62 ""
;;     region17 = 198 ""
;;     region18 = 145 ""
;;     region19 = 139 ""
;;     region20 = 82 ""
;;     region21 = 234 ""
;;     region22 = 124 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64, i64, i64) -> i8 system_v
;;     sig2 = (i64 vmctx, i64, i32, i32, i32) -> i64 tail
;;     sig3 = (i64 vmctx) tail
;;     fn0 = colocated u0:0 sig0
;;     fn1 = colocated u268435456:0 sig1
;;     fn2 = colocated u805306368:41 sig2
;;     fn3 = colocated u805306368:40 sig3
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0047                               v2 = func_addr.i64 fn0
;; @0047                               v3 = func_addr.i64 fn1
;; @0047                               v4 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0047                               v5 = load.i32 notrap aligned readonly can_move region3 v4
;; @0047                               store notrap aligned region4 v3, v0+88
;; @0047                               store notrap aligned region5 v2, v0+96
;; @0047                               store notrap aligned region6 v0, v0+112
;; @0047                               store notrap aligned region1 v5, v0+104
;; @0047                               v6 = iconst.i64 88
;; @0047                               v7 = iadd v0, v6  ; v6 = 88
;; @0049                               trapz v7, user16
;; @0049                               v8 = iconst.i32 0
;; @0049                               v9 = iconst.i32 0
;; @0049                               v10 = iconst.i32 0
;; @0049                               v11 = call fn2(v0, v7, v8, v9, v10)  ; v8 = 0, v9 = 0, v10 = 0
;; @0049                               v12 = load.i64 notrap aligned region7 v11+104
;; @0049                               v13 = uextend.i128 v11
;; @0049                               v14 = uextend.i128 v12
;; @0049                               v15 = iconst.i64 64
;; @0049                               v16 = uextend.i128 v15  ; v15 = 64
;; @0049                               v17 = ishl v14, v16
;; @0049                               v18 = bor v17, v13
;; @004b                               jump block2
;;
;;                                 block2:
;; @004b                               v19 = ireduce.i64 v18
;; @004b                               v20 = iconst.i64 64
;; @004b                               v21 = uextend.i128 v20  ; v20 = 64
;; @004b                               v22 = ushr.i128 v18, v21
;; @004b                               v23 = ireduce.i64 v22
;; @004b                               trapz v19, user16
;; @004b                               v24 = load.i64 notrap aligned region7 v19+104
;; @004b                               v25 = icmp eq v24, v23
;; @004b                               trapz v25, user23
;; @004b                               v26 = iconst.i64 1
;; @004b                               v27 = iadd v24, v26  ; v26 = 1
;; @004b                               store notrap aligned region7 v27, v19+104
;; @004b                               v28 = load.i64 notrap aligned region8 v19+112
;; @004b                               v29 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v30 = load.i64 notrap aligned region9 v29+88
;; @004b                               v31 = load.i64 notrap aligned region9 v29+96
;; @004b                               store notrap aligned region10 v30, v28+88
;; @004b                               store notrap aligned region10 v31, v28+96
;; @004b                               v32 = iconst.i64 0
;; @004b                               store notrap aligned region8 v32, v19+112  ; v32 = 0
;; @004b                               v33 = iconst.i64 2
;; @004b                               v34 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region9 v33, v34+88  ; v33 = 2
;; @004b                               store notrap aligned region9 v19, v34+96
;; @004b                               v35 = iconst.i64 0
;; @004b                               v36 = iadd v19, v35  ; v35 = 0
;; @004b                               v37 = iconst.i32 1
;; @004b                               store notrap aligned region11 v37, v36+32  ; v37 = 1
;; @004b                               v38 = iconst.i32 2
;; @004b                               store notrap aligned region11 v38, v31+32  ; v38 = 2
;; @004b                               v39 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v40 = iconst.i64 0
;; @004b                               v41 = iadd v31, v40  ; v40 = 0
;; @004b                               v42 = load.i64 notrap aligned region12 v39+72
;; @004b                               v43 = load.i64 notrap aligned region13 v39+64
;; @004b                               v44 = load.i64 notrap aligned region14 v39+80
;; @004b                               store notrap aligned region6 v42, v41+8
;; @004b                               store notrap aligned region15 v43, v41+16
;; @004b                               store notrap aligned region16 v44, v41+24
;; @004b                               v45 = load.i64 notrap aligned region1 v39+24
;; @004b                               store notrap aligned region17 v45, v41
;; @004b                               v46 = iconst.i64 0
;; @004b                               v47 = iadd v36, v46  ; v46 = 0
;; @004b                               v48 = load.i64 notrap aligned region17 v47
;; @004b                               store notrap aligned region1 v48, v39+24
;; @004b                               v49 = load.i64 notrap aligned region6 v47+8
;; @004b                               store notrap aligned region12 v49, v39+72
;; @004b                               v50 = load.i64 notrap aligned region15 v47+16
;; @004b                               store notrap aligned region13 v50, v39+64
;; @004b                               v51 = load.i64 notrap aligned region16 v47+24
;; @004b                               store notrap aligned region14 v51, v39+80
;; @004b                               v52 = iconst.i64 40
;; @004b                               v53 = iadd v31, v52  ; v52 = 40
;; @004b                               v54 = iconst.i32 1
;; @004b                               v55 = stack_addr.i64 ss0
;; @004b                               store notrap aligned region18 v54, v53+4  ; v54 = 1
;; @004b                               store notrap aligned region19 v55, v53+8
;; @004b                               v56 = iconst.i64 48
;; @004b                               v57 = iadd.i64 v0, v56  ; v56 = 48
;; @004b                               v58 = iconst.i32 1
;; @004b                               v59 = load.i64 notrap aligned region19 v53+8
;; @004b                               store notrap aligned region20 v57, v59
;; @004b                               store notrap aligned region19 v58, v53  ; v58 = 1
;; @004b                               v60 = iconst.i32 0
;; @004b                               store notrap aligned region21 v60, v31+56  ; v60 = 0
;; @004b                               v61 = iconst.i64 1
;; @004b                               v62 = iconst.i64 32
;; @004b                               v63 = ishl v61, v62  ; v61 = 1, v62 = 32
;; @004b                               v64 = iconst.i64 120
;; @004b                               v65 = iadd v28, v64  ; v64 = 120
;; @004b                               v66 = load.i64 notrap aligned region22 v65
;; @004b                               v67 = iconst.i64 -24
;; @004b                               v68 = iadd v66, v67  ; v67 = -24
;; @004b                               v69 = stack_switch v68, v68, v63
;; @004b                               v70 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v71 = load.i64 notrap aligned region9 v70+88
;; @004b                               v72 = load.i64 notrap aligned region9 v70+96
;; @004b                               v73 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region9 v30, v73+88
;; @004b                               store notrap aligned region9 v31, v73+96
;; @004b                               v74 = iconst.i32 1
;; @004b                               store notrap aligned region11 v74, v31+32  ; v74 = 1
;; @004b                               v75 = iconst.i32 0
;; @004b                               store notrap aligned region19 v75, v53  ; v75 = 0
;; @004b                               v76 = iconst.i32 0
;; @004b                               store notrap aligned region18 v76, v53+4  ; v76 = 0
;; @004b                               v77 = iconst.i64 0
;; @004b                               store notrap aligned region19 v77, v53+8  ; v77 = 0
;; @004b                               store notrap aligned region21 v32, v31+56  ; v32 = 0
;; @004b                               brif v69, block6, block3
;;
;;                                 block6:
;; @004b                               v78 = iconst.i64 32
;; @004b                               v79 = ushr.i64 v69, v78  ; v78 = 32
;; @004b                               v80 = iconst.i64 4
;; @004b                               v81 = icmp eq v79, v80  ; v80 = 4
;; @004b                               brif v81, block5, block4
;;
;;                                 block5 cold:
;; @004b                               v82 = iconst.i64 0
;; @004b                               v83 = iadd.i64 v72, v82  ; v82 = 0
;; @004b                               v84 = iconst.i32 5
;; @004b                               store notrap aligned region11 v84, v83+32  ; v84 = 5
;; @004b                               v85 = iconst.i64 0
;; @004b                               v86 = iadd.i64 v31, v85  ; v85 = 0
;; @004b                               v87 = load.i64 notrap aligned region17 v86
;; @004b                               store notrap aligned region1 v87, v39+24
;; @004b                               v88 = load.i64 notrap aligned region6 v86+8
;; @004b                               store notrap aligned region12 v88, v39+72
;; @004b                               v89 = load.i64 notrap aligned region15 v86+16
;; @004b                               store notrap aligned region13 v89, v39+64
;; @004b                               v90 = load.i64 notrap aligned region16 v86+24
;; @004b                               store notrap aligned region14 v90, v39+80
;; @004b                               v91 = iconst.i64 144
;; @004b                               v92 = iadd.i64 v72, v91  ; v91 = 144
;; @004b                               v93 = iconst.i32 0
;; @004b                               store notrap aligned region19 v93, v92  ; v93 = 0
;; @004b                               v94 = iconst.i32 0
;; @004b                               store notrap aligned region18 v94, v92+4  ; v94 = 0
;; @004b                               v95 = iconst.i64 0
;; @004b                               store notrap aligned region19 v95, v92+8  ; v95 = 0
;; @004b                               v96 = iconst.i64 168
;; @004b                               v97 = iadd.i64 v72, v96  ; v96 = 168
;; @004b                               v98 = iconst.i32 0
;; @004b                               store notrap aligned region19 v98, v97  ; v98 = 0
;; @004b                               v99 = iconst.i32 0
;; @004b                               store notrap aligned region18 v99, v97+4  ; v99 = 0
;; @004b                               v100 = iconst.i64 0
;; @004b                               store notrap aligned region19 v100, v97+8  ; v100 = 0
;; @004b                               try_call fn3(v0), sig3, block8, [ context v0 ]
;;
;;                                 block8:
;; @004b                               trap user12
;;
;;                                 block4:
;; @004b                               v101 = iconst.i64 0
;; @004b                               v102 = iadd.i64 v72, v101  ; v101 = 0
;; @004b                               v103 = iconst.i64 0
;; @004b                               v104 = iadd v102, v103  ; v103 = 0
;; @004b                               v105 = load.i64 notrap aligned region12 v39+72
;; @004b                               v106 = load.i64 notrap aligned region13 v39+64
;; @004b                               v107 = load.i64 notrap aligned region14 v39+80
;; @004b                               store notrap aligned region6 v105, v104+8
;; @004b                               store notrap aligned region15 v106, v104+16
;; @004b                               store notrap aligned region16 v107, v104+24
;; @004b                               v108 = iconst.i64 0
;; @004b                               v109 = iadd.i64 v31, v108  ; v108 = 0
;; @004b                               v110 = load.i64 notrap aligned region17 v109
;; @004b                               store notrap aligned region1 v110, v39+24
;; @004b                               v111 = load.i64 notrap aligned region6 v109+8
;; @004b                               store notrap aligned region12 v111, v39+72
;; @004b                               v112 = load.i64 notrap aligned region15 v109+16
;; @004b                               store notrap aligned region13 v112, v39+64
;; @004b                               v113 = load.i64 notrap aligned region16 v109+24
;; @004b                               store notrap aligned region14 v113, v39+80
;; @004b                               v114 = ireduce.i32 v69
;; @004b                               v115 = load.i64 notrap aligned region7 v72+104
;; @004b                               v116 = uextend.i128 v72
;; @004b                               v117 = uextend.i128 v115
;; @004b                               v118 = iconst.i64 64
;; @004b                               v119 = uextend.i128 v118  ; v118 = 64
;; @004b                               v120 = ishl v117, v119
;; @004b                               v121 = bor v120, v116
;; @004b                               jump block7
;;
;;                                 block9 cold:
;; @004b                               trap user12
;;
;;                                 block7:
;; @004b                               br_table v114, block9, []
;;
;;                                 block3:
;; @004b                               v122 = iconst.i64 0
;; @004b                               v123 = iadd.i64 v31, v122  ; v122 = 0
;; @004b                               v124 = load.i64 notrap aligned region17 v123
;; @004b                               store notrap aligned region1 v124, v39+24
;; @004b                               v125 = load.i64 notrap aligned region6 v123+8
;; @004b                               store notrap aligned region12 v125, v39+72
;; @004b                               v126 = load.i64 notrap aligned region15 v123+16
;; @004b                               store notrap aligned region13 v126, v39+64
;; @004b                               v127 = load.i64 notrap aligned region16 v123+24
;; @004b                               store notrap aligned region14 v127, v39+80
;; @004b                               v128 = iconst.i64 0
;; @004b                               v129 = iadd.i64 v72, v128  ; v128 = 0
;; @004b                               v130 = iconst.i32 4
;; @004b                               store notrap aligned region11 v130, v129+32  ; v130 = 4
;; @004b                               v131 = iconst.i64 144
;; @004b                               v132 = iadd.i64 v72, v131  ; v131 = 144
;; @004b                               v133 = load.i64 notrap aligned region19 v132+8
;; @004b                               v134 = iconst.i32 0
;; @004b                               store notrap aligned region19 v134, v132  ; v134 = 0
;; @004b                               v135 = iconst.i32 0
;; @004b                               store notrap aligned region18 v135, v132+4  ; v135 = 0
;; @004b                               v136 = iconst.i64 0
;; @004b                               store notrap aligned region19 v136, v132+8  ; v136 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
