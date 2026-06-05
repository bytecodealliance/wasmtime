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
      ;; suspend and pass countdown to our consumer
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
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003c                               v3 = iconst.i32 10
;; @0044                               v33 = iconst.i64 120
;; @0044                               v36 = stack_addr.i64 ss0
;; @0044                               v42 = iconst.i64 16
;; @0044                               v39 = iconst.i64 0
;; @0044                               v51 = iconst.i64 80
;; @0044                               v54 = iconst.i64 -24
;;                                     v70 = iconst.i64 0x0002_0000_0000
;; @0040                               jump block2(v3)  ; v3 = 10
;;
;;                                 block2(v4: i32):
;; @0044                               v9 = load.i64 notrap aligned v0+8
;; @0044                               v10 = load.i64 notrap aligned v9+88
;; @0044                               v11 = load.i64 notrap aligned v9+96
;; @0044                               v14 = iconst.i64 1
;; @0044                               v18 = iconst.i64 24
;; @003a                               v2 = iconst.i32 0
;; @0044                               jump block4(v10, v11, v4)
;;
;;                                 block4(v12: i64, v13: i64, v64: i32):
;;                                     v73 = iconst.i64 1
;;                                     v74 = icmp eq v12, v73  ; v73 = 1
;; @0044                               trapnz v74, user22
;; @0044                               jump block5
;;
;;                                 block5:
;; @0044                               v16 = load.i64 notrap aligned v13+48
;; @0044                               v17 = load.i64 notrap aligned v13+56
;;                                     v75 = iconst.i64 24
;;                                     v76 = iadd v17, v75  ; v75 = 24
;; @0044                               v20 = load.i64 notrap aligned v76+8
;; @0044                               v21 = load.i32 notrap aligned v17+40
;;                                     v77 = iconst.i32 0
;;                                     v67 = iconst.i32 3
;; @0044                               v6 = iconst.i64 48
;; @0044                               v7 = iadd.i64 v0, v6  ; v6 = 48
;; @0044                               v31 = iconst.i32 1
;; @0044                               jump block6(v77)  ; v77 = 0
;;
;;                                 block6(v23: i32):
;; @0044                               v24 = icmp ult v23, v21
;; @0044                               brif v24, block7, block4(v16, v17, v64)
;;
;;                                 block7:
;;                                     v78 = iconst.i32 3
;;                                     v79 = ishl.i32 v23, v78  ; v78 = 3
;; @0044                               v27 = uextend.i64 v79
;; @0044                               v28 = iadd.i64 v20, v27
;; @0044                               v29 = load.i64 notrap aligned v28
;;                                     v80 = iadd.i64 v0, v6  ; v6 = 48
;;                                     v81 = icmp eq v29, v80
;;                                     v82 = iconst.i32 1
;;                                     v83 = iadd.i32 v23, v82  ; v82 = 1
;; @0044                               brif v81, block8, block6(v83)
;;
;;                                 block8:
;; @0044                               store.i64 notrap aligned v13, v11+64
;;                                     v84 = iconst.i32 1
;;                                     v85 = iconst.i64 120
;;                                     v86 = iadd.i64 v11, v85  ; v85 = 120
;; @0044                               store notrap aligned v84, v86+4  ; v84 = 1
;; @0044                               store.i64 notrap aligned v36, v86+8
;; @0044                               store.i32 notrap aligned v4, v36
;; @0044                               store notrap aligned v84, v86  ; v84 = 1
;;                                     v87 = iconst.i32 3
;;                                     v88 = iconst.i64 16
;;                                     v89 = iadd.i64 v11, v88  ; v88 = 16
;; @0044                               store notrap aligned v87, v89  ; v87 = 3
;;                                     v90 = iconst.i64 0
;; @0044                               store notrap aligned v90, v13+48  ; v90 = 0
;; @0044                               store notrap aligned v90, v13+56  ; v90 = 0
;;                                     v91 = iconst.i64 80
;;                                     v92 = iadd.i64 v13, v91  ; v91 = 80
;; @0044                               v53 = load.i64 notrap aligned v92
;;                                     v93 = iconst.i64 -24
;;                                     v94 = iadd v53, v93  ; v93 = -24
;; @0044                               v49 = uextend.i64 v23
;;                                     v95 = iconst.i64 0x0002_0000_0000
;;                                     v96 = bor v49, v95  ; v95 = 0x0002_0000_0000
;; @0044                               v56 = stack_switch v94, v94, v96
;; @0044                               v59 = load.i64 notrap aligned v86+8
;;                                     v97 = iconst.i32 0
;; @0044                               store notrap aligned v97, v86  ; v97 = 0
;; @0044                               store notrap aligned v97, v86+4  ; v97 = 0
;; @0044                               store notrap aligned v90, v86+8  ; v90 = 0
;;                                     v98 = isub.i32 v64, v84  ; v84 = 1
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
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0056                               v2 = iconst.i32 0
;; @0056                               v4 = call fn0(v0, v2)  ; v2 = 0
;; @0058                               trapz v4, user16
;; @0058                               v8 = call fn1(v0, v4, v2, v2)  ; v2 = 0, v2 = 0
;; @0058                               v9 = load.i64 notrap aligned v8+72
;; @0058                               v11 = uextend.i128 v9
;; @0058                               v12 = iconst.i64 64
;;                                     v121 = ishl v11, v12  ; v12 = 64
;; @0058                               v10 = uextend.i128 v8
;; @0058                               v15 = bor v121, v10
;; @0062                               v27 = iconst.i64 1
;; @0062                               v33 = iconst.i64 0
;; @0062                               v34 = iconst.i64 2
;; @0062                               v38 = iconst.i32 1
;; @0062                               v39 = iconst.i64 16
;; @0062                               v41 = iconst.i32 2
;; @0062                               v53 = iconst.i64 24
;; @0062                               v56 = stack_addr.i64 ss0
;; @0062                               v58 = iconst.i64 48
;; @0062                               v59 = iadd v0, v58  ; v58 = 48
;; @0062                               v66 = iconst.i64 80
;; @0062                               v69 = iconst.i64 -24
;;                                     v125 = iconst.i64 0x0001_0000_0000
;; @0062                               v64 = iconst.i64 32
;; @005c                               jump block2(v15)
;;
;;                                 block2(v18: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v20 = ireduce.i64 v18
;; @0062                               trapz v20, user16
;; @0062                               v25 = load.i64 notrap aligned v20+72
;;                                     v128 = iconst.i64 64
;;                                     v129 = ushr.i128 v18, v128  ; v128 = 64
;; @0062                               v24 = ireduce.i64 v129
;; @0062                               v26 = icmp eq v25, v24
;; @0062                               trapz v26, user23
;;                                     v130 = iconst.i64 1
;;                                     v131 = iadd v25, v130  ; v130 = 1
;; @0062                               store notrap aligned v131, v20+72
;; @0062                               v29 = load.i64 notrap aligned v20+64
;; @0062                               v30 = load.i64 notrap aligned v0+8
;; @0062                               v31 = load.i64 notrap aligned v30+88
;; @0062                               v32 = load.i64 notrap aligned v30+96
;; @0062                               store notrap aligned v31, v29+48
;; @0062                               store notrap aligned v32, v29+56
;;                                     v132 = iconst.i64 0
;; @0062                               store notrap aligned v132, v20+64  ; v132 = 0
;; @0062                               v35 = load.i64 notrap aligned v0+8
;;                                     v133 = iconst.i64 2
;; @0062                               store notrap aligned v133, v35+88  ; v133 = 2
;; @0062                               store notrap aligned v20, v35+96
;;                                     v134 = iconst.i32 1
;;                                     v135 = iconst.i64 16
;;                                     v136 = iadd v20, v135  ; v135 = 16
;; @0062                               store notrap aligned v134, v136  ; v134 = 1
;;                                     v137 = iconst.i32 2
;;                                     v138 = iadd v32, v135  ; v135 = 16
;; @0062                               store notrap aligned v137, v138  ; v137 = 2
;; @0062                               v44 = load.i64 notrap aligned readonly v0+8
;; @0062                               v47 = load.i64 notrap aligned v44+72
;; @0062                               store notrap aligned v47, v32+8
;; @0062                               v48 = load.i64 notrap aligned v44+24
;; @0062                               store notrap aligned v48, v32
;; @0062                               v51 = load.i64 notrap aligned v20
;; @0062                               store notrap aligned v51, v44+24
;; @0062                               v52 = load.i64 notrap aligned v20+8
;; @0062                               store notrap aligned v52, v44+72
;;                                     v139 = iconst.i64 24
;;                                     v140 = iadd v32, v139  ; v139 = 24
;; @0062                               store notrap aligned v134, v140+4  ; v134 = 1
;; @0062                               store.i64 notrap aligned v56, v140+8
;;                                     v141 = iadd.i64 v0, v58  ; v58 = 48
;; @0062                               store notrap aligned v141, v56
;; @0062                               store notrap aligned v134, v140  ; v134 = 1
;; @0062                               store notrap aligned v134, v32+40  ; v134 = 1
;;                                     v142 = iconst.i64 80
;;                                     v143 = iadd v29, v142  ; v142 = 80
;; @0062                               v68 = load.i64 notrap aligned v143
;;                                     v144 = iconst.i64 -24
;;                                     v145 = iadd v68, v144  ; v144 = -24
;;                                     v146 = iconst.i64 0x0001_0000_0000
;; @0062                               v71 = stack_switch v145, v145, v146  ; v146 = 0x0001_0000_0000
;; @0062                               v72 = load.i64 notrap aligned v0+8
;; @0062                               v73 = load.i64 notrap aligned v72+88
;; @0062                               v74 = load.i64 notrap aligned v72+96
;; @0062                               store notrap aligned v31, v72+88
;; @0062                               store notrap aligned v32, v72+96
;; @0062                               store notrap aligned v134, v138  ; v134 = 1
;;                                     v147 = iconst.i32 0
;; @0062                               store notrap aligned v147, v140  ; v147 = 0
;; @0062                               store notrap aligned v147, v140+4  ; v147 = 0
;; @0062                               store notrap aligned v132, v140+8  ; v132 = 0
;; @0062                               store notrap aligned v132, v32+40  ; v132 = 0
;;                                     v148 = iconst.i64 32
;;                                     v149 = ushr v71, v148  ; v148 = 32
;; @0062                               brif v149, block7, block6
;;
;;                                 block7:
;; @0062                               v88 = load.i64 notrap aligned v44+72
;; @0062                               store notrap aligned v88, v74+8
;; @0062                               v91 = load.i64 notrap aligned v32
;; @0062                               store notrap aligned v91, v44+24
;; @0062                               v92 = load.i64 notrap aligned v32+8
;; @0062                               store notrap aligned v92, v44+72
;; @0062                               v94 = load.i64 notrap aligned v74+72
;; @0062                               jump block8
;;
;;                                 block9 cold:
;; @0062                               trap user12
;;
;;                                 block10:
;; @0062                               v101 = iconst.i64 120
;; @0062                               v102 = iadd.i64 v74, v101  ; v101 = 120
;; @0062                               v103 = load.i64 notrap aligned v102+8
;; @0062                               v104 = load.i32 notrap aligned v103
;;                                     v154 = iconst.i32 0
;; @0062                               store notrap aligned v154, v102  ; v154 = 0
;; @0062                               jump block4
;;
;;                                 block8:
;; @0062                               v93 = ireduce.i32 v71
;; @0062                               br_table v93, block9, [block10]
;;
;;                                 block6:
;; @0062                               v108 = load.i64 notrap aligned v32
;; @0062                               store notrap aligned v108, v44+24
;; @0062                               v109 = load.i64 notrap aligned v32+8
;; @0062                               store notrap aligned v109, v44+72
;; @0062                               v112 = iconst.i32 4
;;                                     v150 = iconst.i64 16
;;                                     v151 = iadd.i64 v74, v150  ; v150 = 16
;; @0062                               store notrap aligned v112, v151  ; v112 = 4
;; @0062                               v115 = iconst.i64 104
;; @0062                               v116 = iadd.i64 v74, v115  ; v115 = 104
;; @0062                               v117 = load.i64 notrap aligned v116+8
;;                                     v152 = iconst.i32 0
;; @0062                               store notrap aligned v152, v116  ; v152 = 0
;; @0062                               store notrap aligned v152, v116+4  ; v152 = 0
;;                                     v153 = iconst.i64 0
;; @0062                               store notrap aligned v153, v116+8  ; v153 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v96 = uextend.i128 v94
;;                                     v155 = iconst.i64 64
;;                                     v156 = ishl v96, v155  ; v155 = 64
;; @0062                               v95 = uextend.i128 v74
;; @0062                               v100 = bor v156, v95
;; @006d                               jump block2(v100)
;; }
