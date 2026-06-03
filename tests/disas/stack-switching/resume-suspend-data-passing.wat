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
;; @0044                               v50 = iconst.i64 80
;; @0044                               v53 = iconst.i64 -24
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
;;                                 block4(v12: i64, v13: i64, v63: i32):
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
;; @0044                               brif v24, block7, block4(v16, v17, v63)
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
;; @0044                               v52 = load.i64 notrap aligned v92
;;                                     v93 = iconst.i64 -24
;;                                     v94 = iadd v52, v93  ; v93 = -24
;; @0044                               v48 = uextend.i64 v23
;;                                     v95 = iconst.i64 0x0002_0000_0000
;;                                     v96 = bor v48, v95  ; v95 = 0x0002_0000_0000
;; @0044                               v55 = stack_switch v94, v94, v96
;; @0044                               v58 = load.i64 notrap aligned v86+8
;;                                     v97 = iconst.i32 0
;; @0044                               store notrap aligned v97, v86  ; v97 = 0
;; @0044                               store notrap aligned v97, v86+4  ; v97 = 0
;; @0044                               store notrap aligned v90, v86+8  ; v90 = 0
;;                                     v98 = isub.i32 v63, v84  ; v84 = 1
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
;;                                     v119 = iconst.i64 64
;;                                     v121 = ishl v11, v119  ; v119 = 64
;; @0058                               v10 = uextend.i128 v8
;; @0058                               v13 = bor v121, v10
;; @0062                               v23 = iconst.i64 1
;; @0062                               v29 = iconst.i64 0
;; @0062                               v30 = iconst.i64 2
;; @0062                               v34 = iconst.i32 1
;; @0062                               v35 = iconst.i64 16
;; @0062                               v37 = iconst.i32 2
;; @0062                               v49 = iconst.i64 24
;; @0062                               v52 = stack_addr.i64 ss0
;; @0062                               v54 = iconst.i64 48
;; @0062                               v55 = iadd v0, v54  ; v54 = 48
;; @0062                               v61 = iconst.i64 80
;; @0062                               v64 = iconst.i64 -24
;;                                     v125 = iconst.i64 0x0001_0000_0000
;;                                     v116 = iconst.i64 32
;; @005c                               jump block2(v13)
;;
;;                                 block2(v16: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v18 = ireduce.i64 v16
;; @0062                               trapz v18, user16
;; @0062                               v21 = load.i64 notrap aligned v18+72
;;                                     v128 = iconst.i64 64
;;                                     v129 = ushr.i128 v16, v128  ; v128 = 64
;; @0062                               v20 = ireduce.i64 v129
;; @0062                               v22 = icmp eq v21, v20
;; @0062                               trapz v22, user23
;;                                     v130 = iconst.i64 1
;;                                     v131 = iadd v21, v130  ; v130 = 1
;; @0062                               store notrap aligned v131, v18+72
;; @0062                               v25 = load.i64 notrap aligned v18+64
;; @0062                               v26 = load.i64 notrap aligned v0+8
;; @0062                               v27 = load.i64 notrap aligned v26+88
;; @0062                               v28 = load.i64 notrap aligned v26+96
;; @0062                               store notrap aligned v27, v25+48
;; @0062                               store notrap aligned v28, v25+56
;;                                     v132 = iconst.i64 0
;; @0062                               store notrap aligned v132, v18+64  ; v132 = 0
;; @0062                               v31 = load.i64 notrap aligned v0+8
;;                                     v133 = iconst.i64 2
;; @0062                               store notrap aligned v133, v31+88  ; v133 = 2
;; @0062                               store notrap aligned v18, v31+96
;;                                     v134 = iconst.i32 1
;;                                     v135 = iconst.i64 16
;;                                     v136 = iadd v18, v135  ; v135 = 16
;; @0062                               store notrap aligned v134, v136  ; v134 = 1
;;                                     v137 = iconst.i32 2
;;                                     v138 = iadd v28, v135  ; v135 = 16
;; @0062                               store notrap aligned v137, v138  ; v137 = 2
;; @0062                               v40 = load.i64 notrap aligned readonly v0+8
;; @0062                               v43 = load.i64 notrap aligned v40+72
;; @0062                               store notrap aligned v43, v28+8
;; @0062                               v44 = load.i64 notrap aligned v40+24
;; @0062                               store notrap aligned v44, v28
;; @0062                               v47 = load.i64 notrap aligned v18
;; @0062                               store notrap aligned v47, v40+24
;; @0062                               v48 = load.i64 notrap aligned v18+8
;; @0062                               store notrap aligned v48, v40+72
;;                                     v139 = iconst.i64 24
;;                                     v140 = iadd v28, v139  ; v139 = 24
;; @0062                               store notrap aligned v134, v140+4  ; v134 = 1
;; @0062                               store.i64 notrap aligned v52, v140+8
;;                                     v141 = iadd.i64 v0, v54  ; v54 = 48
;; @0062                               store notrap aligned v141, v52
;; @0062                               store notrap aligned v134, v140  ; v134 = 1
;; @0062                               store notrap aligned v134, v28+40  ; v134 = 1
;;                                     v142 = iconst.i64 80
;;                                     v143 = iadd v25, v142  ; v142 = 80
;; @0062                               v63 = load.i64 notrap aligned v143
;;                                     v144 = iconst.i64 -24
;;                                     v145 = iadd v63, v144  ; v144 = -24
;;                                     v146 = iconst.i64 0x0001_0000_0000
;; @0062                               v66 = stack_switch v145, v145, v146  ; v146 = 0x0001_0000_0000
;; @0062                               v67 = load.i64 notrap aligned v0+8
;; @0062                               v68 = load.i64 notrap aligned v67+88
;; @0062                               v69 = load.i64 notrap aligned v67+96
;; @0062                               store notrap aligned v27, v67+88
;; @0062                               store notrap aligned v28, v67+96
;; @0062                               store notrap aligned v134, v138  ; v134 = 1
;;                                     v147 = iconst.i32 0
;; @0062                               store notrap aligned v147, v140  ; v147 = 0
;; @0062                               store notrap aligned v147, v140+4  ; v147 = 0
;; @0062                               store notrap aligned v132, v140+8  ; v132 = 0
;; @0062                               store notrap aligned v132, v28+40  ; v132 = 0
;;                                     v148 = iconst.i64 32
;;                                     v149 = ushr v66, v148  ; v148 = 32
;; @0062                               brif v149, block7, block6
;;
;;                                 block7:
;; @0062                               v82 = load.i64 notrap aligned v40+72
;; @0062                               store notrap aligned v82, v69+8
;; @0062                               v85 = load.i64 notrap aligned v28
;; @0062                               store notrap aligned v85, v40+24
;; @0062                               v86 = load.i64 notrap aligned v28+8
;; @0062                               store notrap aligned v86, v40+72
;; @0062                               v88 = load.i64 notrap aligned v69+72
;; @0062                               jump block8
;;
;;                                 block9 cold:
;; @0062                               trap user12
;;
;;                                 block10:
;; @0062                               v93 = iconst.i64 120
;; @0062                               v94 = iadd.i64 v69, v93  ; v93 = 120
;; @0062                               v95 = load.i64 notrap aligned v94+8
;; @0062                               v96 = load.i32 notrap aligned v95
;;                                     v154 = iconst.i32 0
;; @0062                               store notrap aligned v154, v94  ; v154 = 0
;; @0062                               jump block4
;;
;;                                 block8:
;; @0062                               v87 = ireduce.i32 v66
;; @0062                               br_table v87, block9, [block10]
;;
;;                                 block6:
;; @0062                               v100 = load.i64 notrap aligned v28
;; @0062                               store notrap aligned v100, v40+24
;; @0062                               v101 = load.i64 notrap aligned v28+8
;; @0062                               store notrap aligned v101, v40+72
;; @0062                               v104 = iconst.i32 4
;;                                     v150 = iconst.i64 16
;;                                     v151 = iadd.i64 v69, v150  ; v150 = 16
;; @0062                               store notrap aligned v104, v151  ; v104 = 4
;; @0062                               v107 = iconst.i64 104
;; @0062                               v108 = iadd.i64 v69, v107  ; v107 = 104
;; @0062                               v109 = load.i64 notrap aligned v108+8
;;                                     v152 = iconst.i32 0
;; @0062                               store notrap aligned v152, v108  ; v152 = 0
;; @0062                               store notrap aligned v152, v108+4  ; v152 = 0
;;                                     v153 = iconst.i64 0
;; @0062                               store notrap aligned v153, v108+8  ; v153 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v90 = uextend.i128 v88
;;                                     v155 = iconst.i64 64
;;                                     v156 = ishl v90, v155  ; v155 = 64
;; @0062                               v89 = uextend.i128 v69
;; @0062                               v92 = bor v156, v89
;; @006d                               jump block2(v92)
;; }
