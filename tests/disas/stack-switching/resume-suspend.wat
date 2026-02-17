;;! target = "x86_64-unknown-linux-gnu"
;;! flags = "-W stack-switching=y -W exceptions=y -W function-references=y"
;;! test = "optimize"

(module
  (type $ft (func))
  (tag $t (type $ft))
  (type $ct (cont $ft))

  (func $target (suspend $t))
  (elem declare func $target)

  (func (export "minimal_suspend")
    (local $k (ref null $ct))
    (local.set $k (cont.new $ct (ref.func $target)))
    (block $h (result (ref null $ct))
      (resume $ct (on $t $h) (local.get $k))
      ;; continuation suspended back...
      (ref.null $ct)
    )
    (drop)
  )
)

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003b                               v5 = load.i64 notrap aligned v0+8
;; @003b                               v6 = load.i64 notrap aligned v5+88
;; @003b                               v7 = load.i64 notrap aligned v5+96
;;                                     v54 = iconst.i64 1
;;                                     v53 = iconst.i64 24
;; @003b                               v16 = iconst.i32 0
;; @003b                               jump block2(v6, v7)
;;
;;                                 block2(v8: i64, v9: i64):
;;                                     v62 = iconst.i64 1
;;                                     v63 = icmp eq v8, v62  ; v62 = 1
;; @003b                               trapnz v63, user21
;; @003b                               jump block3
;;
;;                                 block3:
;; @003b                               v11 = load.i64 notrap aligned v9+48
;; @003b                               v12 = load.i64 notrap aligned v9+56
;;                                     v64 = iconst.i64 24
;;                                     v65 = iadd v12, v64  ; v64 = 24
;; @003b                               v14 = load.i64 notrap aligned v65+8
;; @003b                               v15 = load.i32 notrap aligned v12+40
;;                                     v66 = iconst.i32 0
;;                                     v56 = iconst.i32 3
;;                                     v55 = iconst.i64 48
;; @003b                               v3 = iadd.i64 v0, v55  ; v55 = 48
;;                                     v51 = iconst.i32 1
;; @003b                               jump block4(v66)  ; v66 = 0
;;
;;                                 block4(v17: i32):
;; @003b                               v18 = icmp ult v17, v15
;; @003b                               brif v18, block5, block2(v11, v12)
;;
;;                                 block5:
;;                                     v67 = iconst.i32 3
;;                                     v68 = ishl.i32 v17, v67  ; v67 = 3
;; @003b                               v20 = uextend.i64 v68
;; @003b                               v21 = iadd.i64 v14, v20
;; @003b                               v22 = load.i64 notrap aligned v21
;;                                     v69 = iadd.i64 v0, v55  ; v55 = 48
;;                                     v70 = icmp eq v22, v69
;;                                     v71 = iconst.i32 1
;;                                     v72 = iadd.i32 v17, v71  ; v71 = 1
;; @003b                               brif v70, block6, block4(v72)
;;
;;                                 block6:
;; @003b                               store.i64 notrap aligned v9, v7+64
;;                                     v73 = iconst.i32 3
;;                                     v48 = iconst.i64 16
;; @003b                               v28 = iadd.i64 v7, v48  ; v48 = 16
;; @003b                               store notrap aligned v73, v28  ; v73 = 3
;;                                     v49 = iconst.i64 0
;; @003b                               store notrap aligned v49, v9+48  ; v49 = 0
;; @003b                               store notrap aligned v49, v9+56  ; v49 = 0
;;                                     v46 = iconst.i64 80
;; @003b                               v35 = iadd.i64 v9, v46  ; v46 = 80
;; @003b                               v36 = load.i64 notrap aligned v35
;;                                     v45 = iconst.i64 -24
;; @003b                               v37 = iadd v36, v45  ; v45 = -24
;; @003b                               v33 = uextend.i64 v17
;;                                     v59 = iconst.i64 0x0002_0000_0000
;;                                     v60 = bor v33, v59  ; v59 = 0x0002_0000_0000
;; @003b                               v38 = stack_switch v37, v37, v60
;;                                     v50 = iconst.i64 120
;; @003b                               v25 = iadd.i64 v7, v50  ; v50 = 120
;; @003b                               v40 = load.i64 notrap aligned v25+8
;;                                     v74 = iconst.i32 0
;; @003b                               store notrap aligned v74, v25  ; v74 = 0
;; @003b                               store notrap aligned v74, v25+4  ; v74 = 0
;; @003b                               store notrap aligned v49, v25+8  ; v49 = 0
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
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
;;     fn0 = colocated u805306368:7 sig0
;;     fn1 = colocated u805306368:52 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0043                               v7 = iconst.i32 0
;; @0043                               v9 = call fn0(v0, v7)  ; v7 = 0
;; @0045                               trapz v9, user15
;; @0045                               v13 = call fn1(v0, v9, v7, v7)  ; v7 = 0, v7 = 0
;; @0045                               v14 = load.i64 notrap aligned v13+72
;; @004e                               jump block3
;;
;;                                 block3:
;; @0045                               v16 = uextend.i128 v14
;;                                     v130 = iconst.i64 64
;;                                     v134 = ishl v16, v130  ; v130 = 64
;;                                     v136 = ireduce.i64 v134
;;                                     v138 = bor v136, v13
;; @004e                               trapz v138, user15
;; @004e                               v24 = load.i64 notrap aligned v138+72
;; @0045                               v15 = uextend.i128 v13
;; @0045                               v18 = bor v134, v15
;;                                     v140 = ushr v18, v130  ; v130 = 64
;; @004e                               v23 = ireduce.i64 v140
;; @004e                               v25 = icmp eq v24, v23
;; @004e                               trapz v25, user22
;;                                     v125 = iconst.i64 1
;; @004e                               v26 = iadd v24, v125  ; v125 = 1
;; @004e                               store notrap aligned v26, v138+72
;; @004e                               v27 = load.i64 notrap aligned v138+64
;; @004e                               v28 = load.i64 notrap aligned v0+8
;; @004e                               v29 = load.i64 notrap aligned v28+88
;; @004e                               v30 = load.i64 notrap aligned v28+96
;; @004e                               store notrap aligned v29, v27+48
;; @004e                               store notrap aligned v30, v27+56
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned v2, v138+64  ; v2 = 0
;; @004e                               v33 = load.i64 notrap aligned v0+8
;; @004e                               v32 = iconst.i64 2
;; @004e                               store notrap aligned v32, v33+88  ; v32 = 2
;; @004e                               store notrap aligned v138, v33+96
;; @004e                               v35 = iconst.i32 1
;;                                     v123 = iconst.i64 16
;; @004e                               v36 = iadd v138, v123  ; v123 = 16
;; @004e                               store notrap aligned v35, v36  ; v35 = 1
;; @004e                               v37 = iconst.i32 2
;; @004e                               v38 = iadd v30, v123  ; v123 = 16
;; @004e                               store notrap aligned v37, v38  ; v37 = 2
;; @004e                               v39 = load.i64 notrap aligned readonly v0+8
;; @004e                               v41 = load.i64 notrap aligned v39+72
;; @004e                               store notrap aligned v41, v30+8
;; @004e                               v42 = load.i64 notrap aligned v39+24
;; @004e                               store notrap aligned v42, v30
;; @004e                               v44 = load.i64 notrap aligned v138
;; @004e                               store notrap aligned v44, v39+24
;; @004e                               v45 = load.i64 notrap aligned v138+8
;; @004e                               store notrap aligned v45, v39+72
;;                                     v119 = iconst.i64 24
;; @004e                               v46 = iadd v30, v119  ; v119 = 24
;; @004e                               store notrap aligned v35, v46+4  ; v35 = 1
;; @004e                               v48 = stack_addr.i64 ss0
;; @004e                               store notrap aligned v48, v46+8
;;                                     v118 = iconst.i64 48
;; @004e                               v50 = iadd.i64 v0, v118  ; v118 = 48
;; @004e                               store notrap aligned v50, v48
;; @004e                               store notrap aligned v35, v46  ; v35 = 1
;; @004e                               store notrap aligned v35, v30+40  ; v35 = 1
;;                                     v116 = iconst.i64 80
;; @004e                               v56 = iadd v27, v116  ; v116 = 80
;; @004e                               v57 = load.i64 notrap aligned v56
;;                                     v115 = iconst.i64 -24
;; @004e                               v58 = iadd v57, v115  ; v115 = -24
;;                                     v142 = iconst.i64 0x0001_0000_0000
;; @004e                               v59 = stack_switch v58, v58, v142  ; v142 = 0x0001_0000_0000
;; @004e                               v60 = load.i64 notrap aligned v0+8
;; @004e                               v61 = load.i64 notrap aligned v60+88
;; @004e                               v62 = load.i64 notrap aligned v60+96
;; @004e                               store notrap aligned v29, v60+88
;; @004e                               store notrap aligned v30, v60+96
;; @004e                               store notrap aligned v35, v38  ; v35 = 1
;;                                     v145 = iconst.i32 0
;; @004e                               store notrap aligned v145, v46  ; v145 = 0
;; @004e                               store notrap aligned v145, v46+4  ; v145 = 0
;; @004e                               store notrap aligned v2, v46+8  ; v2 = 0
;; @004e                               store notrap aligned v2, v30+40  ; v2 = 0
;;                                     v117 = iconst.i64 32
;; @004e                               v69 = ushr v59, v117  ; v117 = 32
;; @004e                               brif v69, block5, block4
;;
;;                                 block5:
;; @004e                               v72 = load.i64 notrap aligned v39+72
;; @004e                               store notrap aligned v72, v62+8
;; @004e                               v74 = load.i64 notrap aligned v30
;; @004e                               store notrap aligned v74, v39+24
;; @004e                               v75 = load.i64 notrap aligned v30+8
;; @004e                               store notrap aligned v75, v39+72
;; @004e                               v77 = load.i64 notrap aligned v62+72
;; @004e                               jump block6
;;
;;                                 block7 cold:
;; @004e                               trap user11
;;
;;                                 block8:
;;                                     v107 = iconst.i64 120
;; @004e                               v82 = iadd.i64 v62, v107  ; v107 = 120
;; @004e                               v83 = load.i64 notrap aligned v82+8
;;                                     v153 = iconst.i32 0
;; @004e                               store notrap aligned v153, v82  ; v153 = 0
;; @004e                               v79 = uextend.i128 v77
;;                                     v154 = iconst.i64 64
;;                                     v155 = ishl v79, v154  ; v154 = 64
;; @004e                               v78 = uextend.i128 v62
;; @004e                               v81 = bor v155, v78
;; @004e                               jump block2(v81)
;;
;;                                 block6:
;; @004e                               v76 = ireduce.i32 v59
;; @004e                               br_table v76, block7, [block8]
;;
;;                                 block4:
;; @004e                               v86 = load.i64 notrap aligned v30
;; @004e                               store notrap aligned v86, v39+24
;; @004e                               v87 = load.i64 notrap aligned v30+8
;; @004e                               store notrap aligned v87, v39+72
;; @004e                               v89 = iconst.i32 4
;;                                     v146 = iconst.i64 16
;;                                     v147 = iadd.i64 v62, v146  ; v146 = 16
;; @004e                               store notrap aligned v89, v147  ; v89 = 4
;;                                     v103 = iconst.i64 104
;; @004e                               v91 = iadd.i64 v62, v103  ; v103 = 104
;; @004e                               v92 = load.i64 notrap aligned v91+8
;;                                     v148 = iconst.i32 0
;; @004e                               store notrap aligned v148, v91  ; v148 = 0
;; @004e                               store notrap aligned v148, v91+4  ; v148 = 0
;;                                     v149 = iconst.i64 0
;; @004e                               store notrap aligned v149, v91+8  ; v149 = 0
;;                                     v150 = uextend.i128 v149  ; v149 = 0
;;                                     v151 = iconst.i64 64
;;                                     v152 = ishl v150, v151  ; v151 = 64
;; @0040                               v6 = bor v152, v150
;; @0056                               jump block2(v6)
;;
;;                                 block2(v19: i128):
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
