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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003c                               v3 = iconst.i32 10
;; @0044                               v31 = iconst.i64 120
;; @0044                               v34 = stack_addr.i64 ss0
;; @0044                               v40 = iconst.i64 16
;; @0044                               v37 = iconst.i64 0
;; @0044                               v49 = iconst.i64 80
;; @0044                               v52 = iconst.i64 -24
;;                                     v68 = iconst.i64 0x0002_0000_0000
;; @0040                               jump block2(v3)  ; v3 = 10
;;
;;                                 block2(v4: i32):
;; @0044                               v7 = load.i64 notrap aligned v0+8
;; @0044                               v8 = load.i64 notrap aligned v7+88
;; @0044                               v9 = load.i64 notrap aligned v7+96
;; @0044                               v12 = iconst.i64 1
;; @0044                               v16 = iconst.i64 24
;; @003a                               v2 = iconst.i32 0
;; @0044                               jump block4(v8, v9, v4)
;;
;;                                 block4(v10: i64, v11: i64, v62: i32):
;;                                     v71 = iconst.i64 1
;;                                     v72 = icmp eq v10, v71  ; v71 = 1
;; @0044                               trapnz v72, user22
;; @0044                               jump block5
;;
;;                                 block5:
;; @0044                               v14 = load.i64 notrap aligned v11+48
;; @0044                               v15 = load.i64 notrap aligned v11+56
;;                                     v73 = iconst.i64 24
;;                                     v74 = iadd v15, v73  ; v73 = 24
;; @0044                               v18 = load.i64 notrap aligned v74+8
;; @0044                               v19 = load.i32 notrap aligned v15+40
;;                                     v75 = iconst.i32 0
;;                                     v65 = iconst.i32 3
;; @0044                               v5 = iconst.i64 48
;; @0044                               v6 = iadd.i64 v0, v5  ; v5 = 48
;; @0044                               v29 = iconst.i32 1
;; @0044                               jump block6(v75)  ; v75 = 0
;;
;;                                 block6(v21: i32):
;; @0044                               v22 = icmp ult v21, v19
;; @0044                               brif v22, block7, block4(v14, v15, v62)
;;
;;                                 block7:
;;                                     v76 = iconst.i32 3
;;                                     v77 = ishl.i32 v21, v76  ; v76 = 3
;; @0044                               v25 = uextend.i64 v77
;; @0044                               v26 = iadd.i64 v18, v25
;; @0044                               v27 = load.i64 notrap aligned v26
;;                                     v78 = iadd.i64 v0, v5  ; v5 = 48
;;                                     v79 = icmp eq v27, v78
;;                                     v80 = iconst.i32 1
;;                                     v81 = iadd.i32 v21, v80  ; v80 = 1
;; @0044                               brif v79, block8, block6(v81)
;;
;;                                 block8:
;; @0044                               store.i64 notrap aligned v11, v9+64
;;                                     v82 = iconst.i32 1
;;                                     v83 = iconst.i64 120
;;                                     v84 = iadd.i64 v9, v83  ; v83 = 120
;; @0044                               store notrap aligned v82, v84+4  ; v82 = 1
;; @0044                               store.i64 notrap aligned v34, v84+8
;; @0044                               store.i32 notrap aligned v4, v34
;; @0044                               store notrap aligned v82, v84  ; v82 = 1
;;                                     v85 = iconst.i32 3
;;                                     v86 = iconst.i64 16
;;                                     v87 = iadd.i64 v9, v86  ; v86 = 16
;; @0044                               store notrap aligned v85, v87  ; v85 = 3
;;                                     v88 = iconst.i64 0
;; @0044                               store notrap aligned v88, v11+48  ; v88 = 0
;; @0044                               store notrap aligned v88, v11+56  ; v88 = 0
;;                                     v89 = iconst.i64 80
;;                                     v90 = iadd.i64 v11, v89  ; v89 = 80
;; @0044                               v51 = load.i64 notrap aligned v90
;;                                     v91 = iconst.i64 -24
;;                                     v92 = iadd v51, v91  ; v91 = -24
;; @0044                               v47 = uextend.i64 v21
;;                                     v93 = iconst.i64 0x0002_0000_0000
;;                                     v94 = bor v47, v93  ; v93 = 0x0002_0000_0000
;; @0044                               v54 = stack_switch v92, v92, v94
;; @0044                               v57 = load.i64 notrap aligned v84+8
;;                                     v95 = iconst.i32 0
;; @0044                               store notrap aligned v95, v84  ; v95 = 0
;; @0044                               store notrap aligned v95, v84+4  ; v95 = 0
;; @0044                               store notrap aligned v88, v84+8  ; v88 = 0
;;                                     v96 = isub.i32 v62, v82  ; v82 = 1
;; @004d                               brif v96, block2(v96), block10
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
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0056                               v2 = iconst.i32 0
;; @0056                               v3 = call fn0(v0, v2)  ; v2 = 0
;; @0058                               trapz v3, user16
;; @0058                               v6 = call fn1(v0, v3, v2, v2)  ; v2 = 0, v2 = 0
;; @0058                               v7 = load.i64 notrap aligned v6+72
;; @0058                               v9 = uextend.i128 v7
;; @0058                               v10 = iconst.i64 64
;;                                     v117 = ishl v9, v10  ; v10 = 64
;; @0058                               v8 = uextend.i128 v6
;; @0058                               v13 = bor v117, v8
;; @0062                               v24 = iconst.i64 1
;; @0062                               v30 = iconst.i64 0
;; @0062                               v31 = iconst.i64 2
;; @0062                               v35 = iconst.i32 1
;; @0062                               v36 = iconst.i64 16
;; @0062                               v38 = iconst.i32 2
;; @0062                               v50 = iconst.i64 24
;; @0062                               v53 = stack_addr.i64 ss0
;; @0062                               v54 = iconst.i64 48
;; @0062                               v55 = iadd v0, v54  ; v54 = 48
;; @0062                               v62 = iconst.i64 80
;; @0062                               v65 = iconst.i64 -24
;;                                     v121 = iconst.i64 0x0001_0000_0000
;; @0062                               v60 = iconst.i64 32
;; @005c                               jump block2(v13)
;;
;;                                 block2(v16: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v17 = ireduce.i64 v16
;; @0062                               trapz v17, user16
;; @0062                               v22 = load.i64 notrap aligned v17+72
;;                                     v124 = iconst.i64 64
;;                                     v125 = ushr.i128 v16, v124  ; v124 = 64
;; @0062                               v21 = ireduce.i64 v125
;; @0062                               v23 = icmp eq v22, v21
;; @0062                               trapz v23, user23
;;                                     v126 = iconst.i64 1
;;                                     v127 = iadd v22, v126  ; v126 = 1
;; @0062                               store notrap aligned v127, v17+72
;; @0062                               v26 = load.i64 notrap aligned v17+64
;; @0062                               v27 = load.i64 notrap aligned v0+8
;; @0062                               v28 = load.i64 notrap aligned v27+88
;; @0062                               v29 = load.i64 notrap aligned v27+96
;; @0062                               store notrap aligned v28, v26+48
;; @0062                               store notrap aligned v29, v26+56
;;                                     v128 = iconst.i64 0
;; @0062                               store notrap aligned v128, v17+64  ; v128 = 0
;; @0062                               v32 = load.i64 notrap aligned v0+8
;;                                     v129 = iconst.i64 2
;; @0062                               store notrap aligned v129, v32+88  ; v129 = 2
;; @0062                               store notrap aligned v17, v32+96
;;                                     v130 = iconst.i32 1
;;                                     v131 = iconst.i64 16
;;                                     v132 = iadd v17, v131  ; v131 = 16
;; @0062                               store notrap aligned v130, v132  ; v130 = 1
;;                                     v133 = iconst.i32 2
;;                                     v134 = iadd v29, v131  ; v131 = 16
;; @0062                               store notrap aligned v133, v134  ; v133 = 2
;; @0062                               v41 = load.i64 notrap aligned readonly v0+8
;; @0062                               v44 = load.i64 notrap aligned v41+72
;; @0062                               store notrap aligned v44, v29+8
;; @0062                               v45 = load.i64 notrap aligned v41+24
;; @0062                               store notrap aligned v45, v29
;; @0062                               v48 = load.i64 notrap aligned v17
;; @0062                               store notrap aligned v48, v41+24
;; @0062                               v49 = load.i64 notrap aligned v17+8
;; @0062                               store notrap aligned v49, v41+72
;;                                     v135 = iconst.i64 24
;;                                     v136 = iadd v29, v135  ; v135 = 24
;; @0062                               store notrap aligned v130, v136+4  ; v130 = 1
;; @0062                               store.i64 notrap aligned v53, v136+8
;;                                     v137 = iadd.i64 v0, v54  ; v54 = 48
;; @0062                               store notrap aligned v137, v53
;; @0062                               store notrap aligned v130, v136  ; v130 = 1
;; @0062                               store notrap aligned v130, v29+40  ; v130 = 1
;;                                     v138 = iconst.i64 80
;;                                     v139 = iadd v26, v138  ; v138 = 80
;; @0062                               v64 = load.i64 notrap aligned v139
;;                                     v140 = iconst.i64 -24
;;                                     v141 = iadd v64, v140  ; v140 = -24
;;                                     v142 = iconst.i64 0x0001_0000_0000
;; @0062                               v67 = stack_switch v141, v141, v142  ; v142 = 0x0001_0000_0000
;; @0062                               v68 = load.i64 notrap aligned v0+8
;; @0062                               v69 = load.i64 notrap aligned v68+88
;; @0062                               v70 = load.i64 notrap aligned v68+96
;; @0062                               store notrap aligned v28, v68+88
;; @0062                               store notrap aligned v29, v68+96
;; @0062                               store notrap aligned v130, v134  ; v130 = 1
;;                                     v143 = iconst.i32 0
;; @0062                               store notrap aligned v143, v136  ; v143 = 0
;; @0062                               store notrap aligned v143, v136+4  ; v143 = 0
;; @0062                               store notrap aligned v128, v136+8  ; v128 = 0
;; @0062                               store notrap aligned v128, v29+40  ; v128 = 0
;;                                     v144 = iconst.i64 32
;;                                     v145 = ushr v67, v144  ; v144 = 32
;; @0062                               brif v145, block7, block6
;;
;;                                 block7:
;; @0062                               v84 = load.i64 notrap aligned v41+72
;; @0062                               store notrap aligned v84, v70+8
;; @0062                               v87 = load.i64 notrap aligned v29
;; @0062                               store notrap aligned v87, v41+24
;; @0062                               v88 = load.i64 notrap aligned v29+8
;; @0062                               store notrap aligned v88, v41+72
;; @0062                               v90 = load.i64 notrap aligned v70+72
;; @0062                               jump block8
;;
;;                                 block9 cold:
;; @0062                               trap user12
;;
;;                                 block10:
;; @0062                               v97 = iconst.i64 120
;; @0062                               v98 = iadd.i64 v70, v97  ; v97 = 120
;; @0062                               v99 = load.i64 notrap aligned v98+8
;; @0062                               v100 = load.i32 notrap aligned v99
;;                                     v150 = iconst.i32 0
;; @0062                               store notrap aligned v150, v98  ; v150 = 0
;; @0062                               jump block4
;;
;;                                 block8:
;; @0062                               v89 = ireduce.i32 v67
;; @0062                               br_table v89, block9, [block10]
;;
;;                                 block6:
;; @0062                               v104 = load.i64 notrap aligned v29
;; @0062                               store notrap aligned v104, v41+24
;; @0062                               v105 = load.i64 notrap aligned v29+8
;; @0062                               store notrap aligned v105, v41+72
;; @0062                               v108 = iconst.i32 4
;;                                     v146 = iconst.i64 16
;;                                     v147 = iadd.i64 v70, v146  ; v146 = 16
;; @0062                               store notrap aligned v108, v147  ; v108 = 4
;; @0062                               v111 = iconst.i64 104
;; @0062                               v112 = iadd.i64 v70, v111  ; v111 = 104
;; @0062                               v113 = load.i64 notrap aligned v112+8
;;                                     v148 = iconst.i32 0
;; @0062                               store notrap aligned v148, v112  ; v148 = 0
;; @0062                               store notrap aligned v148, v112+4  ; v148 = 0
;;                                     v149 = iconst.i64 0
;; @0062                               store notrap aligned v149, v112+8  ; v149 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v92 = uextend.i128 v90
;;                                     v151 = iconst.i64 64
;;                                     v152 = ishl v92, v151  ; v151 = 64
;; @0062                               v91 = uextend.i128 v70
;; @0062                               v96 = bor v152, v91
;; @006d                               jump block2(v96)
;; }
