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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003b                               v4 = load.i64 notrap aligned v0+8
;; @003b                               v5 = load.i64 notrap aligned v4+88
;; @003b                               v6 = load.i64 notrap aligned v4+96
;; @003b                               v9 = iconst.i64 1
;; @003b                               v13 = iconst.i64 24
;; @003b                               v17 = iconst.i32 0
;; @003b                               jump block2(v5, v6)
;;
;;                                 block2(v7: i64, v8: i64):
;;                                     v60 = iconst.i64 1
;;                                     v61 = icmp eq v7, v60  ; v60 = 1
;; @003b                               trapnz v61, user22
;; @003b                               jump block3
;;
;;                                 block3:
;; @003b                               v11 = load.i64 notrap aligned v8+48
;; @003b                               v12 = load.i64 notrap aligned v8+56
;;                                     v62 = iconst.i64 24
;;                                     v63 = iadd v12, v62  ; v62 = 24
;; @003b                               v15 = load.i64 notrap aligned v63+8
;; @003b                               v16 = load.i32 notrap aligned v12+40
;;                                     v64 = iconst.i32 0
;;                                     v54 = iconst.i32 3
;; @003b                               v2 = iconst.i64 48
;; @003b                               v3 = iadd.i64 v0, v2  ; v2 = 48
;; @003b                               v26 = iconst.i32 1
;; @003b                               jump block4(v64)  ; v64 = 0
;;
;;                                 block4(v18: i32):
;; @003b                               v19 = icmp ult v18, v16
;; @003b                               brif v19, block5, block2(v11, v12)
;;
;;                                 block5:
;;                                     v65 = iconst.i32 3
;;                                     v66 = ishl.i32 v18, v65  ; v65 = 3
;; @003b                               v22 = uextend.i64 v66
;; @003b                               v23 = iadd.i64 v15, v22
;; @003b                               v24 = load.i64 notrap aligned v23
;;                                     v67 = iadd.i64 v0, v2  ; v2 = 48
;;                                     v68 = icmp eq v24, v67
;;                                     v69 = iconst.i32 1
;;                                     v70 = iadd.i32 v18, v69  ; v69 = 1
;; @003b                               brif v68, block6, block4(v70)
;;
;;                                 block6:
;; @003b                               store.i64 notrap aligned v8, v6+64
;;                                     v71 = iconst.i32 3
;; @003b                               v33 = iconst.i64 16
;; @003b                               v34 = iadd.i64 v6, v33  ; v33 = 16
;; @003b                               store notrap aligned v71, v34  ; v71 = 3
;; @003b                               v30 = iconst.i64 0
;; @003b                               store notrap aligned v30, v8+48  ; v30 = 0
;; @003b                               store notrap aligned v30, v8+56  ; v30 = 0
;; @003b                               v42 = iconst.i64 80
;; @003b                               v43 = iadd.i64 v8, v42  ; v42 = 80
;; @003b                               v44 = load.i64 notrap aligned v43
;; @003b                               v45 = iconst.i64 -24
;; @003b                               v46 = iadd v44, v45  ; v45 = -24
;; @003b                               v40 = uextend.i64 v18
;;                                     v57 = iconst.i64 0x0002_0000_0000
;;                                     v58 = bor v40, v57  ; v57 = 0x0002_0000_0000
;; @003b                               v47 = stack_switch v46, v46, v58
;; @003b                               v28 = iconst.i64 120
;; @003b                               v29 = iadd.i64 v6, v28  ; v28 = 120
;; @003b                               v50 = load.i64 notrap aligned v29+8
;;                                     v72 = iconst.i32 0
;; @003b                               store notrap aligned v72, v29  ; v72 = 0
;; @003b                               store notrap aligned v72, v29+4  ; v72 = 0
;; @003b                               store notrap aligned v30, v29+8  ; v30 = 0
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
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0043                               v9 = iconst.i32 0
;; @0043                               v10 = call fn0(v0, v9)  ; v9 = 0
;; @0045                               trapz v10, user16
;; @0045                               v13 = call fn1(v0, v10, v9, v9)  ; v9 = 0, v9 = 0
;; @0045                               v14 = load.i64 notrap aligned v13+72
;; @004e                               jump block3
;;
;;                                 block3:
;; @0045                               v16 = uextend.i128 v14
;; @0040                               v5 = iconst.i64 64
;;                                     v130 = ishl v16, v5  ; v5 = 64
;;                                     v132 = ireduce.i64 v130
;;                                     v134 = bor v132, v13
;; @004e                               trapz v134, user16
;; @004e                               v27 = load.i64 notrap aligned v134+72
;; @0045                               v15 = uextend.i128 v13
;; @0045                               v20 = bor v130, v15
;;                                     v136 = ushr v20, v5  ; v5 = 64
;; @004e                               v26 = ireduce.i64 v136
;; @004e                               v28 = icmp eq v27, v26
;; @004e                               trapz v28, user23
;; @004e                               v29 = iconst.i64 1
;; @004e                               v30 = iadd v27, v29  ; v29 = 1
;; @004e                               store notrap aligned v30, v134+72
;; @004e                               v31 = load.i64 notrap aligned v134+64
;; @004e                               v32 = load.i64 notrap aligned v0+8
;; @004e                               v33 = load.i64 notrap aligned v32+88
;; @004e                               v34 = load.i64 notrap aligned v32+96
;; @004e                               store notrap aligned v33, v31+48
;; @004e                               store notrap aligned v34, v31+56
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned v2, v134+64  ; v2 = 0
;; @004e                               v37 = load.i64 notrap aligned v0+8
;; @004e                               v36 = iconst.i64 2
;; @004e                               store notrap aligned v36, v37+88  ; v36 = 2
;; @004e                               store notrap aligned v134, v37+96
;; @004e                               v40 = iconst.i32 1
;; @004e                               v41 = iconst.i64 16
;; @004e                               v42 = iadd v134, v41  ; v41 = 16
;; @004e                               store notrap aligned v40, v42  ; v40 = 1
;; @004e                               v43 = iconst.i32 2
;; @004e                               v45 = iadd v34, v41  ; v41 = 16
;; @004e                               store notrap aligned v43, v45  ; v43 = 2
;; @004e                               v46 = load.i64 notrap aligned readonly v0+8
;; @004e                               v49 = load.i64 notrap aligned v46+72
;; @004e                               store notrap aligned v49, v34+8
;; @004e                               v50 = load.i64 notrap aligned v46+24
;; @004e                               store notrap aligned v50, v34
;; @004e                               v53 = load.i64 notrap aligned v134
;; @004e                               store notrap aligned v53, v46+24
;; @004e                               v54 = load.i64 notrap aligned v134+8
;; @004e                               store notrap aligned v54, v46+72
;; @004e                               v55 = iconst.i64 24
;; @004e                               v56 = iadd v34, v55  ; v55 = 24
;; @004e                               store notrap aligned v40, v56+4  ; v40 = 1
;; @004e                               v58 = stack_addr.i64 ss0
;; @004e                               store notrap aligned v58, v56+8
;; @004e                               v59 = iconst.i64 48
;; @004e                               v60 = iadd.i64 v0, v59  ; v59 = 48
;; @004e                               store notrap aligned v60, v58
;; @004e                               store notrap aligned v40, v56  ; v40 = 1
;; @004e                               store notrap aligned v40, v34+40  ; v40 = 1
;; @004e                               v67 = iconst.i64 80
;; @004e                               v68 = iadd v31, v67  ; v67 = 80
;; @004e                               v69 = load.i64 notrap aligned v68
;; @004e                               v70 = iconst.i64 -24
;; @004e                               v71 = iadd v69, v70  ; v70 = -24
;;                                     v138 = iconst.i64 0x0001_0000_0000
;; @004e                               v72 = stack_switch v71, v71, v138  ; v138 = 0x0001_0000_0000
;; @004e                               v73 = load.i64 notrap aligned v0+8
;; @004e                               v74 = load.i64 notrap aligned v73+88
;; @004e                               v75 = load.i64 notrap aligned v73+96
;; @004e                               store notrap aligned v33, v73+88
;; @004e                               store notrap aligned v34, v73+96
;; @004e                               store notrap aligned v40, v45  ; v40 = 1
;;                                     v141 = iconst.i32 0
;; @004e                               store notrap aligned v141, v56  ; v141 = 0
;; @004e                               store notrap aligned v141, v56+4  ; v141 = 0
;; @004e                               store notrap aligned v2, v56+8  ; v2 = 0
;; @004e                               store notrap aligned v2, v34+40  ; v2 = 0
;; @004e                               v65 = iconst.i64 32
;; @004e                               v84 = ushr v72, v65  ; v65 = 32
;; @004e                               brif v84, block5, block4
;;
;;                                 block5:
;; @004e                               v89 = load.i64 notrap aligned v46+72
;; @004e                               store notrap aligned v89, v75+8
;; @004e                               v92 = load.i64 notrap aligned v34
;; @004e                               store notrap aligned v92, v46+24
;; @004e                               v93 = load.i64 notrap aligned v34+8
;; @004e                               store notrap aligned v93, v46+72
;; @004e                               v95 = load.i64 notrap aligned v75+72
;; @004e                               jump block6
;;
;;                                 block7 cold:
;; @004e                               trap user12
;;
;;                                 block8:
;; @004e                               v102 = iconst.i64 120
;; @004e                               v103 = iadd.i64 v75, v102  ; v102 = 120
;; @004e                               v104 = load.i64 notrap aligned v103+8
;;                                     v149 = iconst.i32 0
;; @004e                               store notrap aligned v149, v103  ; v149 = 0
;; @004e                               v97 = uextend.i128 v95
;;                                     v150 = iconst.i64 64
;;                                     v151 = ishl v97, v150  ; v150 = 64
;; @004e                               v96 = uextend.i128 v75
;; @004e                               v101 = bor v151, v96
;; @004e                               jump block2(v101)
;;
;;                                 block6:
;; @004e                               v94 = ireduce.i32 v72
;; @004e                               br_table v94, block7, [block8]
;;
;;                                 block4:
;; @004e                               v108 = load.i64 notrap aligned v34
;; @004e                               store notrap aligned v108, v46+24
;; @004e                               v109 = load.i64 notrap aligned v34+8
;; @004e                               store notrap aligned v109, v46+72
;; @004e                               v112 = iconst.i32 4
;;                                     v142 = iconst.i64 16
;;                                     v143 = iadd.i64 v75, v142  ; v142 = 16
;; @004e                               store notrap aligned v112, v143  ; v112 = 4
;; @004e                               v115 = iconst.i64 104
;; @004e                               v116 = iadd.i64 v75, v115  ; v115 = 104
;; @004e                               v117 = load.i64 notrap aligned v116+8
;;                                     v144 = iconst.i32 0
;; @004e                               store notrap aligned v144, v116  ; v144 = 0
;; @004e                               store notrap aligned v144, v116+4  ; v144 = 0
;;                                     v145 = iconst.i64 0
;; @004e                               store notrap aligned v145, v116+8  ; v145 = 0
;;                                     v146 = uextend.i128 v145  ; v145 = 0
;;                                     v147 = iconst.i64 64
;;                                     v148 = ishl v146, v147  ; v147 = 64
;; @0040                               v8 = bor v148, v146
;; @0056                               jump block2(v8)
;;
;;                                 block2(v21: i128):
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
