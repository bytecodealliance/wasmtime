;;! target = "x86_64-unknown-linux-gnu"
;;! flags = "-W stack-switching=y -W exceptions=y -W function-references=y"
;;! test = "optimize"

(module
  (type $ft (func))
  (tag $t (param i32))
  (type $ct (cont $ft))

  (func $countdown
    (local $i i32)
    (local.set $i (i32.const 10))
    (loop $loop
      ;; suspend and pass countdown to our cosnumer
      (suspend $t (local.get $i))
      ;; decrement i; break if we're at 0
      (local.tee $i (i32.sub (local.get $i) (i32.const 1)))
      (br_if $loop)
    )
  )
  (elem declare func $countdown)

  (func (export "main")
    (local $c (ref $ct))
    (local.set $c (cont.new $ct (ref.func $countdown)))
    (loop $loop
      (block $on_gen (result i32 (ref $ct))
        (resume $ct (on $t $on_gen) (local.get $c))
        ;; no more data, return
        (return)
      )
      ;; stack contains [i32 (ref $ct)]
      (local.set $c)
      (drop) ;; could print here
      (br $loop)
    )
  )
)

;; function u0:0(i64 vmctx, i64) tail {
;;     ss0 = explicit_slot 16, align = 65536
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003c                               v3 = iconst.i32 10
;;                                     v61 = iconst.i64 120
;; @0044                               v30 = stack_addr.i64 ss0
;;                                     v59 = iconst.i64 16
;;                                     v60 = iconst.i64 0
;;                                     v57 = iconst.i64 80
;;                                     v56 = iconst.i64 -24
;;                                     v70 = iconst.i64 0x0002_0000_0000
;; @0040                               jump block2(v3)  ; v3 = 10
;;
;;                                 block2(v4: i32):
;; @0044                               v8 = load.i64 notrap aligned v0+8
;; @0044                               v9 = load.i64 notrap aligned v8+80
;; @0044                               v10 = load.i64 notrap aligned v8+88
;;                                     v65 = iconst.i64 1
;;                                     v64 = iconst.i64 24
;; @003a                               v2 = iconst.i32 0
;; @0044                               jump block4(v9, v10, v4)
;;
;;                                 block4(v11: i64, v12: i64, v52: i32):
;;                                     v73 = iconst.i64 1
;;                                     v74 = icmp eq v11, v73  ; v73 = 1
;; @0044                               trapnz v74, user21
;; @0044                               jump block5
;;
;;                                 block5:
;; @0044                               v14 = load.i64 notrap aligned v12+48
;; @0044                               v15 = load.i64 notrap aligned v12+56
;;                                     v75 = iconst.i64 24
;;                                     v76 = iadd v15, v75  ; v75 = 24
;; @0044                               v17 = load.i64 notrap aligned v76+8
;; @0044                               v18 = load.i32 notrap aligned v15+40
;;                                     v77 = iconst.i32 0
;;                                     v67 = iconst.i32 3
;;                                     v66 = iconst.i64 48
;; @0044                               v6 = iadd.i64 v0, v66  ; v66 = 48
;;                                     v62 = iconst.i32 1
;; @0044                               jump block6(v77)  ; v77 = 0
;;
;;                                 block6(v20: i32):
;; @0044                               v21 = icmp ult v20, v18
;; @0044                               brif v21, block7, block4(v14, v15, v52)
;;
;;                                 block7:
;;                                     v78 = iconst.i32 3
;;                                     v79 = ishl.i32 v20, v78  ; v78 = 3
;; @0044                               v23 = uextend.i64 v79
;; @0044                               v24 = iadd.i64 v17, v23
;; @0044                               v25 = load.i64 notrap aligned v24
;;                                     v80 = iadd.i64 v0, v66  ; v66 = 48
;;                                     v81 = icmp eq v25, v80
;;                                     v82 = iconst.i32 1
;;                                     v83 = iadd.i32 v20, v82  ; v82 = 1
;; @0044                               brif v81, block8, block6(v83)
;;
;;                                 block8:
;; @0044                               store.i64 notrap aligned v12, v10+64
;;                                     v84 = iconst.i32 1
;;                                     v85 = iconst.i64 120
;;                                     v86 = iadd.i64 v10, v85  ; v85 = 120
;; @0044                               store notrap aligned v84, v86+4  ; v84 = 1
;; @0044                               store.i64 notrap aligned v30, v86+8
;; @0044                               store.i32 notrap aligned v4, v30
;; @0044                               store notrap aligned v84, v86  ; v84 = 1
;;                                     v87 = iconst.i32 3
;;                                     v88 = iconst.i64 16
;;                                     v89 = iadd.i64 v10, v88  ; v88 = 16
;; @0044                               store notrap aligned v87, v89  ; v87 = 3
;;                                     v90 = iconst.i64 0
;; @0044                               store notrap aligned v90, v12+48  ; v90 = 0
;; @0044                               store notrap aligned v90, v12+56  ; v90 = 0
;;                                     v91 = iconst.i64 80
;;                                     v92 = iadd.i64 v12, v91  ; v91 = 80
;; @0044                               v43 = load.i64 notrap aligned v92
;;                                     v93 = iconst.i64 -24
;;                                     v94 = iadd v43, v93  ; v93 = -24
;; @0044                               v40 = uextend.i64 v20
;;                                     v95 = iconst.i64 0x0002_0000_0000
;;                                     v96 = bor v40, v95  ; v95 = 0x0002_0000_0000
;; @0044                               v45 = stack_switch v94, v94, v96
;; @0044                               v47 = load.i64 notrap aligned v86+8
;;                                     v97 = iconst.i32 0
;; @0044                               store notrap aligned v97, v86  ; v97 = 0
;; @0044                               store notrap aligned v97, v86+4  ; v97 = 0
;; @0044                               store notrap aligned v90, v86+8  ; v90 = 0
;;                                     v98 = isub.i32 v52, v84  ; v84 = 1
;; @004d                               brif v98, block2(v98), block10
;;
;;                                 block10:
;; @004f                               jump block3
;;
;;                                 block3:
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
;;
;; function u0:1(i64 vmctx, i64) tail {
;;     ss0 = explicit_slot 8, align = 256
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     fn1 = colocated u805306368:52 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0056                               v2 = iconst.i32 0
;; @0056                               v4 = call fn0(v0, v2)  ; v2 = 0
;; @0058                               trapz v4, user15
;; @0058                               v8 = call fn1(v0, v4, v2, v2)  ; v2 = 0, v2 = 0
;; @0058                               v9 = load.i64 notrap aligned v8+72
;; @0058                               v11 = uextend.i128 v9
;;                                     v119 = iconst.i64 64
;;                                     v121 = ishl v11, v119  ; v119 = 64
;; @0058                               v10 = uextend.i128 v8
;; @0058                               v13 = bor v121, v10
;;                                     v116 = iconst.i64 1
;; @0062                               v28 = iconst.i64 0
;; @0062                               v29 = iconst.i64 2
;; @0062                               v32 = iconst.i32 1
;;                                     v114 = iconst.i64 16
;; @0062                               v34 = iconst.i32 2
;;                                     v110 = iconst.i64 24
;; @0062                               v45 = stack_addr.i64 ss0
;;                                     v109 = iconst.i64 48
;; @0062                               v47 = iadd v0, v109  ; v109 = 48
;;                                     v107 = iconst.i64 80
;;                                     v106 = iconst.i64 -24
;;                                     v125 = iconst.i64 0x0001_0000_0000
;;                                     v108 = iconst.i64 32
;; @005c                               jump block2(v13)
;;
;;                                 block2(v16: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v18 = ireduce.i64 v16
;; @0062                               trapz v18, user15
;; @0062                               v21 = load.i64 notrap aligned v18+72
;;                                     v128 = iconst.i64 64
;;                                     v129 = ushr.i128 v16, v128  ; v128 = 64
;; @0062                               v20 = ireduce.i64 v129
;; @0062                               v22 = icmp eq v21, v20
;; @0062                               trapz v22, user22
;;                                     v130 = iconst.i64 1
;;                                     v131 = iadd v21, v130  ; v130 = 1
;; @0062                               store notrap aligned v131, v18+72
;; @0062                               v24 = load.i64 notrap aligned v18+64
;; @0062                               v25 = load.i64 notrap aligned v0+8
;; @0062                               v26 = load.i64 notrap aligned v25+80
;; @0062                               v27 = load.i64 notrap aligned v25+88
;; @0062                               store notrap aligned v26, v24+48
;; @0062                               store notrap aligned v27, v24+56
;;                                     v132 = iconst.i64 0
;; @0062                               store notrap aligned v132, v18+64  ; v132 = 0
;; @0062                               v30 = load.i64 notrap aligned v0+8
;;                                     v133 = iconst.i64 2
;; @0062                               store notrap aligned v133, v30+80  ; v133 = 2
;; @0062                               store notrap aligned v18, v30+88
;;                                     v134 = iconst.i32 1
;;                                     v135 = iconst.i64 16
;;                                     v136 = iadd v18, v135  ; v135 = 16
;; @0062                               store notrap aligned v134, v136  ; v134 = 1
;;                                     v137 = iconst.i32 2
;;                                     v138 = iadd v27, v135  ; v135 = 16
;; @0062                               store notrap aligned v137, v138  ; v137 = 2
;; @0062                               v36 = load.i64 notrap aligned readonly v0+8
;; @0062                               v38 = load.i64 notrap aligned v36+64
;; @0062                               store notrap aligned v38, v27+8
;; @0062                               v39 = load.i64 notrap aligned v36+16
;; @0062                               store notrap aligned v39, v27
;; @0062                               v41 = load.i64 notrap aligned v18
;; @0062                               store notrap aligned v41, v36+16
;; @0062                               v42 = load.i64 notrap aligned v18+8
;; @0062                               store notrap aligned v42, v36+64
;;                                     v139 = iconst.i64 24
;;                                     v140 = iadd v27, v139  ; v139 = 24
;; @0062                               store notrap aligned v134, v140+4  ; v134 = 1
;; @0062                               store.i64 notrap aligned v45, v140+8
;;                                     v141 = iadd.i64 v0, v109  ; v109 = 48
;; @0062                               store notrap aligned v141, v45
;; @0062                               store notrap aligned v134, v140  ; v134 = 1
;; @0062                               store notrap aligned v134, v27+40  ; v134 = 1
;;                                     v142 = iconst.i64 80
;;                                     v143 = iadd v24, v142  ; v142 = 80
;; @0062                               v54 = load.i64 notrap aligned v143
;;                                     v144 = iconst.i64 -24
;;                                     v145 = iadd v54, v144  ; v144 = -24
;;                                     v146 = iconst.i64 0x0001_0000_0000
;; @0062                               v56 = stack_switch v145, v145, v146  ; v146 = 0x0001_0000_0000
;; @0062                               v57 = load.i64 notrap aligned v0+8
;; @0062                               v58 = load.i64 notrap aligned v57+80
;; @0062                               v59 = load.i64 notrap aligned v57+88
;; @0062                               store notrap aligned v26, v57+80
;; @0062                               store notrap aligned v27, v57+88
;; @0062                               store notrap aligned v134, v138  ; v134 = 1
;;                                     v147 = iconst.i32 0
;; @0062                               store notrap aligned v147, v140  ; v147 = 0
;; @0062                               store notrap aligned v147, v140+4  ; v147 = 0
;; @0062                               store notrap aligned v132, v140+8  ; v132 = 0
;; @0062                               store notrap aligned v132, v27+40  ; v132 = 0
;;                                     v148 = iconst.i64 32
;;                                     v149 = ushr v56, v148  ; v148 = 32
;; @0062                               brif v149, block7, block6
;;
;;                                 block7:
;; @0062                               v69 = load.i64 notrap aligned v36+64
;; @0062                               store notrap aligned v69, v59+8
;; @0062                               v71 = load.i64 notrap aligned v27
;; @0062                               store notrap aligned v71, v36+16
;; @0062                               v72 = load.i64 notrap aligned v27+8
;; @0062                               store notrap aligned v72, v36+64
;; @0062                               v74 = load.i64 notrap aligned v59+72
;; @0062                               jump block8
;;
;;                                 block9 cold:
;; @0062                               trap user11
;;
;;                                 block10:
;;                                     v98 = iconst.i64 120
;; @0062                               v79 = iadd.i64 v59, v98  ; v98 = 120
;; @0062                               v80 = load.i64 notrap aligned v79+8
;; @0062                               v81 = load.i32 notrap aligned v80
;;                                     v154 = iconst.i32 0
;; @0062                               store notrap aligned v154, v79  ; v154 = 0
;; @0062                               jump block4
;;
;;                                 block8:
;; @0062                               v73 = ireduce.i32 v56
;; @0062                               br_table v73, block9, [block10]
;;
;;                                 block6:
;; @0062                               v84 = load.i64 notrap aligned v27
;; @0062                               store notrap aligned v84, v36+16
;; @0062                               v85 = load.i64 notrap aligned v27+8
;; @0062                               store notrap aligned v85, v36+64
;; @0062                               v87 = iconst.i32 4
;;                                     v150 = iconst.i64 16
;;                                     v151 = iadd.i64 v59, v150  ; v150 = 16
;; @0062                               store notrap aligned v87, v151  ; v87 = 4
;;                                     v94 = iconst.i64 104
;; @0062                               v89 = iadd.i64 v59, v94  ; v94 = 104
;; @0062                               v90 = load.i64 notrap aligned v89+8
;;                                     v152 = iconst.i32 0
;; @0062                               store notrap aligned v152, v89  ; v152 = 0
;; @0062                               store notrap aligned v152, v89+4  ; v152 = 0
;;                                     v153 = iconst.i64 0
;; @0062                               store notrap aligned v153, v89+8  ; v153 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v76 = uextend.i128 v74
;;                                     v155 = iconst.i64 64
;;                                     v156 = ishl v76, v155  ; v155 = 64
;; @0062                               v75 = uextend.i128 v59
;; @0062                               v78 = bor v156, v75
;; @006d                               jump block2(v78)
;; }
