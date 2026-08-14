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
;;     region8 = 2415919112 "VMStackLimits+0x8"
;;     region9 = 2415919120 "VMStackLimits+0x10"
;;     region10 = 2415919128 "VMStackLimits+0x18"
;;     region11 = 2415919104 "VMStackLimits+0x0"
;;     region12 = 1543503873 "Stack(ss1)"
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
;; @003e                               v32 = load.i64 notrap aligned region2 v29+64
;; @003e                               v33 = load.i64 notrap aligned region2 v29+72
;; @003e                               v34 = iconst.i64 40
;; @003e                               v35 = iadd v33, v34  ; v34 = 40
;; @003e                               v36 = load.i64 notrap aligned region2 v35+8
;; @003e                               v37 = load.i32 notrap aligned region2 v33+56
;; @003e                               v38 = load.i32 notrap aligned region2 v35
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
;; @003e                               v45 = load.i64 notrap aligned region4 v44
;; @003e                               v46 = icmp eq v45, v24
;; @003e                               v47 = iconst.i32 1
;; @003e                               v48 = iadd.i32 v39, v47  ; v47 = 1
;; @003e                               brif v46, block6, block4(v48)
;;
;;                                 block7 cold:
;; @003e                               trap user22
;;
;;                                 block6:
;; @003e                               store.i64 notrap aligned region2 v29, v27+80
;; @003e                               v49 = iconst.i64 136
;; @003e                               v50 = iadd.i64 v27, v49  ; v49 = 136
;; @003e                               v51 = iconst.i32 1
;; @003e                               v52 = stack_addr.i64 ss0
;; @003e                               store notrap aligned region2 v51, v50+4  ; v51 = 1
;; @003e                               store notrap aligned region2 v52, v50+8
;; @003e                               v53 = iconst.i64 0
;; @003e                               v54 = iadd.i64 v27, v53  ; v53 = 0
;; @003e                               v55 = iconst.i32 3
;; @003e                               v56 = iconst.i64 32
;; @003e                               v57 = iadd v54, v56  ; v56 = 32
;; @003e                               store notrap aligned region2 v55, v57  ; v55 = 3
;; @003e                               v58 = iconst.i64 0
;; @003e                               v59 = iconst.i64 0
;; @003e                               store notrap aligned region2 v58, v29+64  ; v58 = 0
;; @003e                               store notrap aligned region2 v59, v29+72  ; v59 = 0
;; @003e                               v60 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v61 = iconst.i64 0
;; @003e                               v62 = iadd v54, v61  ; v61 = 0
;; @003e                               v63 = load.i64 notrap aligned region5 v60+72
;; @003e                               v64 = load.i64 notrap aligned region6 v60+64
;; @003e                               v65 = load.i64 notrap aligned region7 v60+80
;; @003e                               store notrap aligned region8 v63, v62+8
;; @003e                               store notrap aligned region9 v64, v62+16
;; @003e                               store notrap aligned region10 v65, v62+24
;; @003e                               v66 = load.i64 notrap aligned region2 v27+88
;; @003e                               v67 = uextend.i128 v27
;; @003e                               v68 = uextend.i128 v66
;; @003e                               v69 = iconst.i64 64
;; @003e                               v70 = uextend.i128 v69  ; v69 = 64
;; @003e                               v71 = ishl v68, v70
;; @003e                               v72 = bor v71, v67
;; @003e                               v74 = iconst.i64 0
;; @003e                               v75 = iadd.i64 v14, v74  ; v74 = 0
;; @003e                               v76 = iconst.i64 32
;; @003e                               v77 = iadd v75, v76  ; v76 = 32
;; @003e                               v78 = load.i32 notrap aligned region2 v77
;; @003e                               v79 = iconst.i32 0
;; @003e                               v80 = icmp ne v78, v79  ; v79 = 0
;; @003e                               brif v80, block9, block8
;;
;;                                 block8:
;; @003e                               v81 = iconst.i64 120
;; @003e                               v82 = iadd.i64 v14, v81  ; v81 = 120
;; @003e                               v83 = load.i64 notrap aligned region2 v82+8
;; @003e                               v84 = load.i32 notrap aligned region2 v82
;; @003e                               v85 = iconst.i32 1
;; @003e                               v86 = iadd v84, v85  ; v85 = 1
;; @003e                               store notrap aligned region2 v86, v82
;; @003e                               v87 = uextend.i64 v84
;; @003e                               v88 = iconst.i64 16
;; @003e                               v89 = imul v87, v88  ; v88 = 16
;; @003e                               v90 = iadd v83, v89
;; @003e                               jump block10(v90)
;;
;;                                 block9:
;; @003e                               v91 = iconst.i64 136
;; @003e                               v92 = iadd.i64 v14, v91  ; v91 = 136
;; @003e                               v93 = load.i64 notrap aligned region2 v92+8
;; @003e                               v94 = load.i32 notrap aligned region2 v92
;; @003e                               v95 = iconst.i32 1
;; @003e                               v96 = iadd v94, v95  ; v95 = 1
;; @003e                               store notrap aligned region2 v96, v92
;; @003e                               v97 = uextend.i64 v94
;; @003e                               v98 = iconst.i64 16
;; @003e                               v99 = imul v97, v98  ; v98 = 16
;; @003e                               v100 = iadd v93, v99
;; @003e                               jump block10(v100)
;;
;;                                 block10(v73: i64):
;; @003e                               store.i128 notrap aligned region4 v72, v73
;; @003e                               v101 = iconst.i64 0
;; @003e                               v102 = iadd.i64 v14, v101  ; v101 = 0
;; @003e                               v103 = iconst.i32 1
;; @003e                               v104 = iconst.i64 32
;; @003e                               v105 = iadd v102, v104  ; v104 = 32
;; @003e                               store notrap aligned region2 v103, v105  ; v103 = 1
;; @003e                               v106 = load.i64 notrap aligned region2 v14+80
;; @003e                               store.i64 notrap aligned region2 v32, v106+64
;; @003e                               store.i64 notrap aligned region2 v33, v106+72
;; @003e                               v107 = iconst.i64 2
;; @003e                               v108 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               store notrap aligned region3 v107, v108+88  ; v107 = 2
;; @003e                               store.i64 notrap aligned region3 v14, v108+96
;; @003e                               v109 = iconst.i64 0
;; @003e                               v110 = iadd v102, v109  ; v109 = 0
;; @003e                               v111 = load.i64 notrap aligned region11 v110
;; @003e                               store notrap aligned region1 v111, v60+24
;; @003e                               v112 = load.i64 notrap aligned region8 v110+8
;; @003e                               store notrap aligned region5 v112, v60+72
;; @003e                               v113 = load.i64 notrap aligned region9 v110+16
;; @003e                               store notrap aligned region6 v113, v60+64
;; @003e                               v114 = load.i64 notrap aligned region10 v110+24
;; @003e                               store notrap aligned region7 v114, v60+80
;; @003e                               v115 = iconst.i64 96
;; @003e                               v116 = iadd.i64 v29, v115  ; v115 = 96
;; @003e                               v117 = load.i64 notrap aligned region2 v116
;; @003e                               v118 = iconst.i64 -24
;; @003e                               v119 = iadd v117, v118  ; v118 = -24
;; @003e                               v120 = iconst.i64 96
;; @003e                               v121 = iadd v106, v120  ; v120 = 96
;; @003e                               v122 = load.i64 notrap aligned region2 v121
;; @003e                               v123 = iconst.i64 -24
;; @003e                               v124 = iadd v122, v123  ; v123 = -24
;; @003e                               v125 = stack_addr.i64 ss1
;; @003e                               v126 = load.i64 notrap aligned region4 v124
;; @003e                               store notrap aligned region12 v126, v125
;; @003e                               v127 = load.i64 notrap aligned region4 v119
;; @003e                               store notrap aligned region4 v127, v124
;; @003e                               v128 = load.i64 notrap aligned region4 v124+8
;; @003e                               store notrap aligned region12 v128, v125+8
;; @003e                               v129 = load.i64 notrap aligned region4 v119+8
;; @003e                               store notrap aligned region4 v129, v124+8
;; @003e                               v130 = load.i64 notrap aligned region4 v124+16
;; @003e                               store notrap aligned region12 v130, v125+16
;; @003e                               v131 = load.i64 notrap aligned region4 v119+16
;; @003e                               store notrap aligned region4 v131, v124+16
;; @003e                               v132 = iconst.i64 3
;; @003e                               v133 = iconst.i64 32
;; @003e                               v134 = ishl v132, v133  ; v132 = 3, v133 = 32
;; @003e                               v135 = stack_switch v119, v125, v134
;; @003e                               v136 = iconst.i64 32
;; @003e                               v137 = ushr v135, v136  ; v136 = 32
;; @003e                               v138 = iconst.i64 5
;; @003e                               v139 = icmp eq v137, v138  ; v138 = 5
;; @003e                               brif v139, block11, block12
;;
;;                                 block11 cold:
;; @003e                               v140 = iconst.i64 136
;; @003e                               v141 = iadd.i64 v27, v140  ; v140 = 136
;; @003e                               v142 = load.i64 notrap aligned region2 v141+8
;; @003e                               v143 = load.i32 notrap aligned region4 v142
;; @003e                               v144 = iconst.i32 0
;; @003e                               store notrap aligned region2 v144, v141  ; v144 = 0
;; @003e                               v145 = iconst.i32 0
;; @003e                               store notrap aligned region2 v145, v141+4  ; v145 = 0
;; @003e                               v146 = iconst.i64 0
;; @003e                               store notrap aligned region2 v146, v141+8  ; v146 = 0
;; @003e                               try_call fn2(v0, v143), sig2, block13, [ context v0 ]
;;
;;                                 block13:
;; @003e                               trap user12
;;
;;                                 block12:
;; @003e                               v147 = iconst.i64 136
;; @003e                               v148 = iadd.i64 v27, v147  ; v147 = 136
;; @003e                               v149 = load.i64 notrap aligned region2 v148+8
;; @003e                               v150 = iconst.i32 0
;; @003e                               store notrap aligned region2 v150, v148  ; v150 = 0
;; @003e                               v151 = iconst.i32 0
;; @003e                               store notrap aligned region2 v151, v148+4  ; v151 = 0
;; @003e                               v152 = iconst.i64 0
;; @003e                               store notrap aligned region2 v152, v148+8  ; v152 = 0
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
;;     region7 = 2415919112 "VMStackLimits+0x8"
;;     region8 = 2415919120 "VMStackLimits+0x10"
;;     region9 = 2415919128 "VMStackLimits+0x18"
;;     region10 = 2415919104 "VMStackLimits+0x0"
;;     region11 = 1140850688 "ContinuationStackMemory+0x0"
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
;; @004b                               v23 = load.i64 notrap aligned region2 v14+80
;; @004b                               v24 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v25 = load.i64 notrap aligned region3 v24+88
;; @004b                               v26 = load.i64 notrap aligned region3 v24+96
;; @004b                               store notrap aligned region2 v25, v23+64
;; @004b                               store notrap aligned region2 v26, v23+72
;; @004b                               v27 = iconst.i64 0
;; @004b                               store notrap aligned region2 v27, v14+80  ; v27 = 0
;; @004b                               v28 = iconst.i64 2
;; @004b                               v29 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region3 v28, v29+88  ; v28 = 2
;; @004b                               store notrap aligned region3 v14, v29+96
;; @004b                               v30 = iconst.i64 0
;; @004b                               v31 = iadd v14, v30  ; v30 = 0
;; @004b                               v32 = iconst.i32 1
;; @004b                               v33 = iconst.i64 32
;; @004b                               v34 = iadd v31, v33  ; v33 = 32
;; @004b                               store notrap aligned region2 v32, v34  ; v32 = 1
;; @004b                               v35 = iconst.i32 2
;; @004b                               v36 = iconst.i64 32
;; @004b                               v37 = iadd v26, v36  ; v36 = 32
;; @004b                               store notrap aligned region2 v35, v37  ; v35 = 2
;; @004b                               v38 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v39 = iconst.i64 0
;; @004b                               v40 = iadd v26, v39  ; v39 = 0
;; @004b                               v41 = load.i64 notrap aligned region4 v38+72
;; @004b                               v42 = load.i64 notrap aligned region5 v38+64
;; @004b                               v43 = load.i64 notrap aligned region6 v38+80
;; @004b                               store notrap aligned region7 v41, v40+8
;; @004b                               store notrap aligned region8 v42, v40+16
;; @004b                               store notrap aligned region9 v43, v40+24
;; @004b                               v44 = load.i64 notrap aligned region1 v38+24
;; @004b                               store notrap aligned region10 v44, v40
;; @004b                               v45 = iconst.i64 0
;; @004b                               v46 = iadd v31, v45  ; v45 = 0
;; @004b                               v47 = load.i64 notrap aligned region10 v46
;; @004b                               store notrap aligned region1 v47, v38+24
;; @004b                               v48 = load.i64 notrap aligned region7 v46+8
;; @004b                               store notrap aligned region4 v48, v38+72
;; @004b                               v49 = load.i64 notrap aligned region8 v46+16
;; @004b                               store notrap aligned region5 v49, v38+64
;; @004b                               v50 = load.i64 notrap aligned region9 v46+24
;; @004b                               store notrap aligned region6 v50, v38+80
;; @004b                               v51 = iconst.i64 40
;; @004b                               v52 = iadd v26, v51  ; v51 = 40
;; @004b                               v53 = iconst.i32 1
;; @004b                               v54 = stack_addr.i64 ss0
;; @004b                               store notrap aligned region2 v53, v52+4  ; v53 = 1
;; @004b                               store notrap aligned region2 v54, v52+8
;; @004b                               v55 = iconst.i64 48
;; @004b                               v56 = iadd.i64 v0, v55  ; v55 = 48
;; @004b                               v57 = iconst.i32 1
;; @004b                               v58 = load.i64 notrap aligned region2 v52+8
;; @004b                               store notrap aligned region11 v56, v58
;; @004b                               store notrap aligned region2 v57, v52  ; v57 = 1
;; @004b                               v59 = iconst.i32 0
;; @004b                               store notrap aligned region2 v59, v26+56  ; v59 = 0
;; @004b                               v60 = iconst.i64 1
;; @004b                               v61 = iconst.i64 32
;; @004b                               v62 = ishl v60, v61  ; v60 = 1, v61 = 32
;; @004b                               v63 = iconst.i64 96
;; @004b                               v64 = iadd v23, v63  ; v63 = 96
;; @004b                               v65 = load.i64 notrap aligned region2 v64
;; @004b                               v66 = iconst.i64 -24
;; @004b                               v67 = iadd v65, v66  ; v66 = -24
;; @004b                               v68 = stack_switch v67, v67, v62
;; @004b                               v69 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               v70 = load.i64 notrap aligned region3 v69+88
;; @004b                               v71 = load.i64 notrap aligned region3 v69+96
;; @004b                               v72 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004b                               store notrap aligned region3 v25, v72+88
;; @004b                               store notrap aligned region3 v26, v72+96
;; @004b                               v73 = iconst.i32 1
;; @004b                               v74 = iconst.i64 32
;; @004b                               v75 = iadd v26, v74  ; v74 = 32
;; @004b                               store notrap aligned region2 v73, v75  ; v73 = 1
;; @004b                               v76 = iconst.i32 0
;; @004b                               store notrap aligned region2 v76, v52  ; v76 = 0
;; @004b                               v77 = iconst.i32 0
;; @004b                               store notrap aligned region2 v77, v52+4  ; v77 = 0
;; @004b                               v78 = iconst.i64 0
;; @004b                               store notrap aligned region2 v78, v52+8  ; v78 = 0
;; @004b                               store notrap aligned region2 v27, v26+56  ; v27 = 0
;; @004b                               brif v68, block6, block3
;;
;;                                 block6:
;; @004b                               v79 = iconst.i64 32
;; @004b                               v80 = ushr.i64 v68, v79  ; v79 = 32
;; @004b                               v81 = iconst.i64 4
;; @004b                               v82 = icmp eq v80, v81  ; v81 = 4
;; @004b                               brif v82, block5, block4
;;
;;                                 block5 cold:
;; @004b                               v83 = iconst.i64 0
;; @004b                               v84 = iadd.i64 v71, v83  ; v83 = 0
;; @004b                               v85 = iconst.i32 5
;; @004b                               v86 = iconst.i64 32
;; @004b                               v87 = iadd v84, v86  ; v86 = 32
;; @004b                               store notrap aligned region2 v85, v87  ; v85 = 5
;; @004b                               v88 = iconst.i64 0
;; @004b                               v89 = iadd.i64 v26, v88  ; v88 = 0
;; @004b                               v90 = load.i64 notrap aligned region10 v89
;; @004b                               store notrap aligned region1 v90, v38+24
;; @004b                               v91 = load.i64 notrap aligned region7 v89+8
;; @004b                               store notrap aligned region4 v91, v38+72
;; @004b                               v92 = load.i64 notrap aligned region8 v89+16
;; @004b                               store notrap aligned region5 v92, v38+64
;; @004b                               v93 = load.i64 notrap aligned region9 v89+24
;; @004b                               store notrap aligned region6 v93, v38+80
;; @004b                               v94 = iconst.i64 120
;; @004b                               v95 = iadd.i64 v71, v94  ; v94 = 120
;; @004b                               v96 = iconst.i32 0
;; @004b                               store notrap aligned region2 v96, v95  ; v96 = 0
;; @004b                               v97 = iconst.i32 0
;; @004b                               store notrap aligned region2 v97, v95+4  ; v97 = 0
;; @004b                               v98 = iconst.i64 0
;; @004b                               store notrap aligned region2 v98, v95+8  ; v98 = 0
;; @004b                               v99 = iconst.i64 136
;; @004b                               v100 = iadd.i64 v71, v99  ; v99 = 136
;; @004b                               v101 = iconst.i32 0
;; @004b                               store notrap aligned region2 v101, v100  ; v101 = 0
;; @004b                               v102 = iconst.i32 0
;; @004b                               store notrap aligned region2 v102, v100+4  ; v102 = 0
;; @004b                               v103 = iconst.i64 0
;; @004b                               store notrap aligned region2 v103, v100+8  ; v103 = 0
;; @004b                               try_call fn2(v0), sig2, block8, [ context v0 ]
;;
;;                                 block8:
;; @004b                               trap user12
;;
;;                                 block4:
;; @004b                               v104 = iconst.i64 0
;; @004b                               v105 = iadd.i64 v71, v104  ; v104 = 0
;; @004b                               v106 = iconst.i64 0
;; @004b                               v107 = iadd v105, v106  ; v106 = 0
;; @004b                               v108 = load.i64 notrap aligned region4 v38+72
;; @004b                               v109 = load.i64 notrap aligned region5 v38+64
;; @004b                               v110 = load.i64 notrap aligned region6 v38+80
;; @004b                               store notrap aligned region7 v108, v107+8
;; @004b                               store notrap aligned region8 v109, v107+16
;; @004b                               store notrap aligned region9 v110, v107+24
;; @004b                               v111 = iconst.i64 0
;; @004b                               v112 = iadd.i64 v26, v111  ; v111 = 0
;; @004b                               v113 = load.i64 notrap aligned region10 v112
;; @004b                               store notrap aligned region1 v113, v38+24
;; @004b                               v114 = load.i64 notrap aligned region7 v112+8
;; @004b                               store notrap aligned region4 v114, v38+72
;; @004b                               v115 = load.i64 notrap aligned region8 v112+16
;; @004b                               store notrap aligned region5 v115, v38+64
;; @004b                               v116 = load.i64 notrap aligned region9 v112+24
;; @004b                               store notrap aligned region6 v116, v38+80
;; @004b                               v117 = ireduce.i32 v68
;; @004b                               v118 = load.i64 notrap aligned region2 v71+88
;; @004b                               v119 = uextend.i128 v71
;; @004b                               v120 = uextend.i128 v118
;; @004b                               v121 = iconst.i64 64
;; @004b                               v122 = uextend.i128 v121  ; v121 = 64
;; @004b                               v123 = ishl v120, v122
;; @004b                               v124 = bor v123, v119
;; @004b                               jump block7
;;
;;                                 block9 cold:
;; @004b                               trap user12
;;
;;                                 block7:
;; @004b                               br_table v117, block9, []
;;
;;                                 block3:
;; @004b                               v125 = iconst.i64 0
;; @004b                               v126 = iadd.i64 v26, v125  ; v125 = 0
;; @004b                               v127 = load.i64 notrap aligned region10 v126
;; @004b                               store notrap aligned region1 v127, v38+24
;; @004b                               v128 = load.i64 notrap aligned region7 v126+8
;; @004b                               store notrap aligned region4 v128, v38+72
;; @004b                               v129 = load.i64 notrap aligned region8 v126+16
;; @004b                               store notrap aligned region5 v129, v38+64
;; @004b                               v130 = load.i64 notrap aligned region9 v126+24
;; @004b                               store notrap aligned region6 v130, v38+80
;; @004b                               v131 = iconst.i64 0
;; @004b                               v132 = iadd.i64 v71, v131  ; v131 = 0
;; @004b                               v133 = iconst.i32 4
;; @004b                               v134 = iconst.i64 32
;; @004b                               v135 = iadd v132, v134  ; v134 = 32
;; @004b                               store notrap aligned region2 v133, v135  ; v133 = 4
;; @004b                               v136 = iconst.i64 120
;; @004b                               v137 = iadd.i64 v71, v136  ; v136 = 120
;; @004b                               v138 = load.i64 notrap aligned region2 v137+8
;; @004b                               v139 = iconst.i32 0
;; @004b                               store notrap aligned region2 v139, v137  ; v139 = 0
;; @004b                               v140 = iconst.i32 0
;; @004b                               store notrap aligned region2 v140, v137+4  ; v140 = 0
;; @004b                               v141 = iconst.i64 0
;; @004b                               store notrap aligned region2 v141, v137+8  ; v141 = 0
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
