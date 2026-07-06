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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108952 "VMStoreContext+0x58"
;;     region3 = 1073741824 "VMContRef+0x0"
;;     region4 = 1140850688 "ContinuationStackMemory+0x0"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003c                               v3 = iconst.i32 10
;; @0044                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
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
;; @0044                               v8 = load.i64 notrap aligned region2 v7+88
;; @0044                               v9 = load.i64 notrap aligned region2 v7+96
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
;; @0044                               v14 = load.i64 notrap aligned region3 v11+48
;; @0044                               v15 = load.i64 notrap aligned region3 v11+56
;;                                     v73 = iconst.i64 24
;;                                     v74 = iadd v15, v73  ; v73 = 24
;; @0044                               v18 = load.i64 notrap aligned region3 v74+8
;; @0044                               v19 = load.i32 notrap aligned region3 v15+40
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
;; @0044                               v27 = load.i64 notrap aligned region4 v26
;;                                     v78 = iadd.i64 v0, v5  ; v5 = 48
;;                                     v79 = icmp eq v27, v78
;;                                     v80 = iconst.i32 1
;;                                     v81 = iadd.i32 v21, v80  ; v80 = 1
;; @0044                               brif v79, block8, block6(v81)
;;
;;                                 block8:
;; @0044                               store.i64 notrap aligned region3 v11, v9+64
;;                                     v82 = iconst.i32 1
;;                                     v83 = iconst.i64 120
;;                                     v84 = iadd.i64 v9, v83  ; v83 = 120
;; @0044                               store notrap aligned region3 v82, v84+4  ; v82 = 1
;; @0044                               store.i64 notrap aligned region3 v34, v84+8
;; @0044                               store.i32 notrap aligned region4 v4, v34
;; @0044                               store notrap aligned region3 v82, v84  ; v82 = 1
;;                                     v85 = iconst.i32 3
;;                                     v86 = iconst.i64 16
;;                                     v87 = iadd.i64 v9, v86  ; v86 = 16
;; @0044                               store notrap aligned region3 v85, v87  ; v85 = 3
;;                                     v88 = iconst.i64 0
;; @0044                               store notrap aligned region3 v88, v11+48  ; v88 = 0
;; @0044                               store notrap aligned region3 v88, v11+56  ; v88 = 0
;;                                     v89 = iconst.i64 80
;;                                     v90 = iadd.i64 v11, v89  ; v89 = 80
;; @0044                               v51 = load.i64 notrap aligned region3 v90
;;                                     v91 = iconst.i64 -24
;;                                     v92 = iadd v51, v91  ; v91 = -24
;; @0044                               v47 = uextend.i64 v21
;;                                     v93 = iconst.i64 0x0002_0000_0000
;;                                     v94 = bor v47, v93  ; v93 = 0x0002_0000_0000
;; @0044                               v54 = stack_switch v92, v92, v94
;; @0044                               v57 = load.i64 notrap aligned region3 v84+8
;;                                     v95 = iconst.i32 0
;; @0044                               store notrap aligned region3 v95, v84  ; v95 = 0
;; @0044                               store notrap aligned region3 v95, v84+4  ; v95 = 0
;; @0044                               store notrap aligned region3 v88, v84+8  ; v88 = 0
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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1073741824 "VMContRef+0x0"
;;     region3 = 67108952 "VMStoreContext+0x58"
;;     region4 = 67108936 "VMStoreContext+0x48"
;;     region5 = 1140850688 "ContinuationStackMemory+0x0"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
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
;; @0058                               v7 = load.i64 notrap aligned region2 v6+72
;; @0058                               v9 = uextend.i128 v7
;; @0058                               v10 = iconst.i64 64
;;                                     v115 = ishl v9, v10  ; v10 = 64
;; @0058                               v8 = uextend.i128 v6
;; @0058                               v13 = bor v115, v8
;; @0062                               v22 = iconst.i64 1
;; @0062                               v25 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0062                               v28 = iconst.i64 0
;; @0062                               v29 = iconst.i64 2
;; @0062                               v33 = iconst.i32 1
;; @0062                               v34 = iconst.i64 16
;; @0062                               v36 = iconst.i32 2
;; @0062                               v48 = iconst.i64 24
;; @0062                               v51 = stack_addr.i64 ss0
;; @0062                               v52 = iconst.i64 48
;; @0062                               v53 = iadd v0, v52  ; v52 = 48
;; @0062                               v60 = iconst.i64 80
;; @0062                               v63 = iconst.i64 -24
;;                                     v119 = iconst.i64 0x0001_0000_0000
;; @0062                               v58 = iconst.i64 32
;; @005c                               jump block2(v13)
;;
;;                                 block2(v14: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v15 = ireduce.i64 v14
;; @0062                               trapz v15, user16
;; @0062                               v20 = load.i64 notrap aligned region2 v15+72
;;                                     v122 = iconst.i64 64
;;                                     v123 = ushr.i128 v14, v122  ; v122 = 64
;; @0062                               v19 = ireduce.i64 v123
;; @0062                               v21 = icmp eq v20, v19
;; @0062                               trapz v21, user23
;;                                     v124 = iconst.i64 1
;;                                     v125 = iadd v20, v124  ; v124 = 1
;; @0062                               store notrap aligned region2 v125, v15+72
;; @0062                               v24 = load.i64 notrap aligned region2 v15+64
;; @0062                               v26 = load.i64 notrap aligned region3 v25+88
;; @0062                               v27 = load.i64 notrap aligned region3 v25+96
;; @0062                               store notrap aligned region2 v26, v24+48
;; @0062                               store notrap aligned region2 v27, v24+56
;;                                     v126 = iconst.i64 0
;; @0062                               store notrap aligned region2 v126, v15+64  ; v126 = 0
;;                                     v127 = iconst.i64 2
;; @0062                               store notrap aligned region3 v127, v25+88  ; v127 = 2
;; @0062                               store notrap aligned region3 v15, v25+96
;;                                     v128 = iconst.i32 1
;;                                     v129 = iconst.i64 16
;;                                     v130 = iadd v15, v129  ; v129 = 16
;; @0062                               store notrap aligned region2 v128, v130  ; v128 = 1
;;                                     v131 = iconst.i32 2
;;                                     v132 = iadd v27, v129  ; v129 = 16
;; @0062                               store notrap aligned region2 v131, v132  ; v131 = 2
;; @0062                               v42 = load.i64 notrap aligned region4 v25+72
;; @0062                               store notrap aligned region2 v42, v27+8
;; @0062                               v43 = load.i64 notrap aligned region1 v25+24
;; @0062                               store notrap aligned region2 v43, v27
;; @0062                               v46 = load.i64 notrap aligned region2 v15
;; @0062                               store notrap aligned region1 v46, v25+24
;; @0062                               v47 = load.i64 notrap aligned region2 v15+8
;; @0062                               store notrap aligned region4 v47, v25+72
;;                                     v133 = iconst.i64 24
;;                                     v134 = iadd v27, v133  ; v133 = 24
;; @0062                               store notrap aligned region2 v128, v134+4  ; v128 = 1
;; @0062                               store.i64 notrap aligned region2 v51, v134+8
;;                                     v135 = iadd.i64 v0, v52  ; v52 = 48
;; @0062                               store notrap aligned region5 v135, v51
;; @0062                               store notrap aligned region2 v128, v134  ; v128 = 1
;; @0062                               store notrap aligned region2 v128, v27+40  ; v128 = 1
;;                                     v136 = iconst.i64 80
;;                                     v137 = iadd v24, v136  ; v136 = 80
;; @0062                               v62 = load.i64 notrap aligned region2 v137
;;                                     v138 = iconst.i64 -24
;;                                     v139 = iadd v62, v138  ; v138 = -24
;;                                     v140 = iconst.i64 0x0001_0000_0000
;; @0062                               v65 = stack_switch v139, v139, v140  ; v140 = 0x0001_0000_0000
;; @0062                               v67 = load.i64 notrap aligned region3 v25+88
;; @0062                               v68 = load.i64 notrap aligned region3 v25+96
;; @0062                               store notrap aligned region3 v26, v25+88
;; @0062                               store notrap aligned region3 v27, v25+96
;; @0062                               store notrap aligned region2 v128, v132  ; v128 = 1
;;                                     v141 = iconst.i32 0
;; @0062                               store notrap aligned region2 v141, v134  ; v141 = 0
;; @0062                               store notrap aligned region2 v141, v134+4  ; v141 = 0
;; @0062                               store notrap aligned region2 v126, v134+8  ; v126 = 0
;; @0062                               store notrap aligned region2 v126, v27+40  ; v126 = 0
;;                                     v142 = iconst.i64 32
;;                                     v143 = ushr v65, v142  ; v142 = 32
;; @0062                               brif v143, block7, block6
;;
;;                                 block7:
;; @0062                               v82 = load.i64 notrap aligned region4 v25+72
;; @0062                               store notrap aligned region2 v82, v68+8
;; @0062                               v85 = load.i64 notrap aligned region2 v27
;; @0062                               store notrap aligned region1 v85, v25+24
;; @0062                               v86 = load.i64 notrap aligned region2 v27+8
;; @0062                               store notrap aligned region4 v86, v25+72
;; @0062                               v88 = load.i64 notrap aligned region2 v68+72
;; @0062                               jump block8
;;
;;                                 block9 cold:
;; @0062                               trap user12
;;
;;                                 block10:
;; @0062                               v95 = iconst.i64 120
;; @0062                               v96 = iadd.i64 v68, v95  ; v95 = 120
;; @0062                               v97 = load.i64 notrap aligned region2 v96+8
;; @0062                               v98 = load.i32 notrap aligned region5 v97
;;                                     v148 = iconst.i32 0
;; @0062                               store notrap aligned region2 v148, v96  ; v148 = 0
;; @0062                               jump block4
;;
;;                                 block8:
;; @0062                               v87 = ireduce.i32 v65
;; @0062                               br_table v87, block9, [block10]
;;
;;                                 block6:
;; @0062                               v102 = load.i64 notrap aligned region2 v27
;; @0062                               store notrap aligned region1 v102, v25+24
;; @0062                               v103 = load.i64 notrap aligned region2 v27+8
;; @0062                               store notrap aligned region4 v103, v25+72
;; @0062                               v106 = iconst.i32 4
;;                                     v144 = iconst.i64 16
;;                                     v145 = iadd.i64 v68, v144  ; v144 = 16
;; @0062                               store notrap aligned region2 v106, v145  ; v106 = 4
;; @0062                               v109 = iconst.i64 104
;; @0062                               v110 = iadd.i64 v68, v109  ; v109 = 104
;; @0062                               v111 = load.i64 notrap aligned region2 v110+8
;;                                     v146 = iconst.i32 0
;; @0062                               store notrap aligned region2 v146, v110  ; v146 = 0
;; @0062                               store notrap aligned region2 v146, v110+4  ; v146 = 0
;;                                     v147 = iconst.i64 0
;; @0062                               store notrap aligned region2 v147, v110+8  ; v147 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v90 = uextend.i128 v88
;;                                     v149 = iconst.i64 64
;;                                     v150 = ishl v90, v149  ; v149 = 64
;; @0062                               v89 = uextend.i128 v68
;; @0062                               v94 = bor v150, v89
;; @006d                               jump block2(v94)
;; }
