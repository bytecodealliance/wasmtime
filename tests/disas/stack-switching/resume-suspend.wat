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
;; @003b                               v6 = load.i64 notrap aligned v0+8
;; @003b                               v7 = load.i64 notrap aligned v6+88
;; @003b                               v8 = load.i64 notrap aligned v6+96
;; @003b                               v11 = iconst.i64 1
;; @003b                               v15 = iconst.i64 24
;; @003b                               v19 = iconst.i32 0
;; @003b                               jump block2(v7, v8)
;;
;;                                 block2(v9: i64, v10: i64):
;;                                     v62 = iconst.i64 1
;;                                     v63 = icmp eq v9, v62  ; v62 = 1
;; @003b                               trapnz v63, user22
;; @003b                               jump block3
;;
;;                                 block3:
;; @003b                               v13 = load.i64 notrap aligned v10+48
;; @003b                               v14 = load.i64 notrap aligned v10+56
;;                                     v64 = iconst.i64 24
;;                                     v65 = iadd v14, v64  ; v64 = 24
;; @003b                               v17 = load.i64 notrap aligned v65+8
;; @003b                               v18 = load.i32 notrap aligned v14+40
;;                                     v66 = iconst.i32 0
;;                                     v56 = iconst.i32 3
;; @003b                               v3 = iconst.i64 48
;; @003b                               v4 = iadd.i64 v0, v3  ; v3 = 48
;; @003b                               v28 = iconst.i32 1
;; @003b                               jump block4(v66)  ; v66 = 0
;;
;;                                 block4(v20: i32):
;; @003b                               v21 = icmp ult v20, v18
;; @003b                               brif v21, block5, block2(v13, v14)
;;
;;                                 block5:
;;                                     v67 = iconst.i32 3
;;                                     v68 = ishl.i32 v20, v67  ; v67 = 3
;; @003b                               v24 = uextend.i64 v68
;; @003b                               v25 = iadd.i64 v17, v24
;; @003b                               v26 = load.i64 notrap aligned v25
;;                                     v69 = iadd.i64 v0, v3  ; v3 = 48
;;                                     v70 = icmp eq v26, v69
;;                                     v71 = iconst.i32 1
;;                                     v72 = iadd.i32 v20, v71  ; v71 = 1
;; @003b                               brif v70, block6, block4(v72)
;;
;;                                 block6:
;; @003b                               store.i64 notrap aligned v10, v8+64
;;                                     v73 = iconst.i32 3
;; @003b                               v35 = iconst.i64 16
;; @003b                               v36 = iadd.i64 v8, v35  ; v35 = 16
;; @003b                               store notrap aligned v73, v36  ; v73 = 3
;; @003b                               v32 = iconst.i64 0
;; @003b                               store notrap aligned v32, v10+48  ; v32 = 0
;; @003b                               store notrap aligned v32, v10+56  ; v32 = 0
;; @003b                               v44 = iconst.i64 80
;; @003b                               v45 = iadd.i64 v10, v44  ; v44 = 80
;; @003b                               v46 = load.i64 notrap aligned v45
;; @003b                               v47 = iconst.i64 -24
;; @003b                               v48 = iadd v46, v47  ; v47 = -24
;; @003b                               v42 = uextend.i64 v20
;;                                     v59 = iconst.i64 0x0002_0000_0000
;;                                     v60 = bor v42, v59  ; v59 = 0x0002_0000_0000
;; @003b                               v49 = stack_switch v48, v48, v60
;; @003b                               v30 = iconst.i64 120
;; @003b                               v31 = iadd.i64 v8, v30  ; v30 = 120
;; @003b                               v52 = load.i64 notrap aligned v31+8
;;                                     v74 = iconst.i32 0
;; @003b                               store notrap aligned v74, v31  ; v74 = 0
;; @003b                               store notrap aligned v74, v31+4  ; v74 = 0
;; @003b                               store notrap aligned v32, v31+8  ; v32 = 0
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
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0043                               v9 = iconst.i32 0
;; @0043                               v11 = call fn0(v0, v9)  ; v9 = 0
;; @0045                               trapz v11, user16
;; @0045                               v15 = call fn1(v0, v11, v9, v9)  ; v9 = 0, v9 = 0
;; @0045                               v16 = load.i64 notrap aligned v15+72
;; @004e                               jump block3
;;
;;                                 block3:
;; @0045                               v18 = uextend.i128 v16
;; @0040                               v5 = iconst.i64 64
;;                                     v134 = ishl v18, v5  ; v5 = 64
;;                                     v136 = ireduce.i64 v134
;;                                     v138 = bor v136, v15
;; @004e                               trapz v138, user16
;; @004e                               v30 = load.i64 notrap aligned v138+72
;; @0045                               v17 = uextend.i128 v15
;; @0045                               v22 = bor v134, v17
;;                                     v140 = ushr v22, v5  ; v5 = 64
;; @004e                               v29 = ireduce.i64 v140
;; @004e                               v31 = icmp eq v30, v29
;; @004e                               trapz v31, user23
;; @004e                               v32 = iconst.i64 1
;; @004e                               v33 = iadd v30, v32  ; v32 = 1
;; @004e                               store notrap aligned v33, v138+72
;; @004e                               v34 = load.i64 notrap aligned v138+64
;; @004e                               v35 = load.i64 notrap aligned v0+8
;; @004e                               v36 = load.i64 notrap aligned v35+88
;; @004e                               v37 = load.i64 notrap aligned v35+96
;; @004e                               store notrap aligned v36, v34+48
;; @004e                               store notrap aligned v37, v34+56
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned v2, v138+64  ; v2 = 0
;; @004e                               v40 = load.i64 notrap aligned v0+8
;; @004e                               v39 = iconst.i64 2
;; @004e                               store notrap aligned v39, v40+88  ; v39 = 2
;; @004e                               store notrap aligned v138, v40+96
;; @004e                               v43 = iconst.i32 1
;; @004e                               v44 = iconst.i64 16
;; @004e                               v45 = iadd v138, v44  ; v44 = 16
;; @004e                               store notrap aligned v43, v45  ; v43 = 1
;; @004e                               v46 = iconst.i32 2
;; @004e                               v48 = iadd v37, v44  ; v44 = 16
;; @004e                               store notrap aligned v46, v48  ; v46 = 2
;; @004e                               v49 = load.i64 notrap aligned readonly v0+8
;; @004e                               v52 = load.i64 notrap aligned v49+72
;; @004e                               store notrap aligned v52, v37+8
;; @004e                               v53 = load.i64 notrap aligned v49+24
;; @004e                               store notrap aligned v53, v37
;; @004e                               v56 = load.i64 notrap aligned v138
;; @004e                               store notrap aligned v56, v49+24
;; @004e                               v57 = load.i64 notrap aligned v138+8
;; @004e                               store notrap aligned v57, v49+72
;; @004e                               v58 = iconst.i64 24
;; @004e                               v59 = iadd v37, v58  ; v58 = 24
;; @004e                               store notrap aligned v43, v59+4  ; v43 = 1
;; @004e                               v61 = stack_addr.i64 ss0
;; @004e                               store notrap aligned v61, v59+8
;; @004e                               v63 = iconst.i64 48
;; @004e                               v64 = iadd.i64 v0, v63  ; v63 = 48
;; @004e                               store notrap aligned v64, v61
;; @004e                               store notrap aligned v43, v59  ; v43 = 1
;; @004e                               store notrap aligned v43, v37+40  ; v43 = 1
;; @004e                               v71 = iconst.i64 80
;; @004e                               v72 = iadd v34, v71  ; v71 = 80
;; @004e                               v73 = load.i64 notrap aligned v72
;; @004e                               v74 = iconst.i64 -24
;; @004e                               v75 = iadd v73, v74  ; v74 = -24
;;                                     v142 = iconst.i64 0x0001_0000_0000
;; @004e                               v76 = stack_switch v75, v75, v142  ; v142 = 0x0001_0000_0000
;; @004e                               v77 = load.i64 notrap aligned v0+8
;; @004e                               v78 = load.i64 notrap aligned v77+88
;; @004e                               v79 = load.i64 notrap aligned v77+96
;; @004e                               store notrap aligned v36, v77+88
;; @004e                               store notrap aligned v37, v77+96
;; @004e                               store notrap aligned v43, v48  ; v43 = 1
;;                                     v145 = iconst.i32 0
;; @004e                               store notrap aligned v145, v59  ; v145 = 0
;; @004e                               store notrap aligned v145, v59+4  ; v145 = 0
;; @004e                               store notrap aligned v2, v59+8  ; v2 = 0
;; @004e                               store notrap aligned v2, v37+40  ; v2 = 0
;; @004e                               v69 = iconst.i64 32
;; @004e                               v88 = ushr v76, v69  ; v69 = 32
;; @004e                               brif v88, block5, block4
;;
;;                                 block5:
;; @004e                               v93 = load.i64 notrap aligned v49+72
;; @004e                               store notrap aligned v93, v79+8
;; @004e                               v96 = load.i64 notrap aligned v37
;; @004e                               store notrap aligned v96, v49+24
;; @004e                               v97 = load.i64 notrap aligned v37+8
;; @004e                               store notrap aligned v97, v49+72
;; @004e                               v99 = load.i64 notrap aligned v79+72
;; @004e                               jump block6
;;
;;                                 block7 cold:
;; @004e                               trap user12
;;
;;                                 block8:
;; @004e                               v106 = iconst.i64 120
;; @004e                               v107 = iadd.i64 v79, v106  ; v106 = 120
;; @004e                               v108 = load.i64 notrap aligned v107+8
;;                                     v153 = iconst.i32 0
;; @004e                               store notrap aligned v153, v107  ; v153 = 0
;; @004e                               v101 = uextend.i128 v99
;;                                     v154 = iconst.i64 64
;;                                     v155 = ishl v101, v154  ; v154 = 64
;; @004e                               v100 = uextend.i128 v79
;; @004e                               v105 = bor v155, v100
;; @004e                               jump block2(v105)
;;
;;                                 block6:
;; @004e                               v98 = ireduce.i32 v76
;; @004e                               br_table v98, block7, [block8]
;;
;;                                 block4:
;; @004e                               v112 = load.i64 notrap aligned v37
;; @004e                               store notrap aligned v112, v49+24
;; @004e                               v113 = load.i64 notrap aligned v37+8
;; @004e                               store notrap aligned v113, v49+72
;; @004e                               v116 = iconst.i32 4
;;                                     v146 = iconst.i64 16
;;                                     v147 = iadd.i64 v79, v146  ; v146 = 16
;; @004e                               store notrap aligned v116, v147  ; v116 = 4
;; @004e                               v119 = iconst.i64 104
;; @004e                               v120 = iadd.i64 v79, v119  ; v119 = 104
;; @004e                               v121 = load.i64 notrap aligned v120+8
;;                                     v148 = iconst.i32 0
;; @004e                               store notrap aligned v148, v120  ; v148 = 0
;; @004e                               store notrap aligned v148, v120+4  ; v148 = 0
;;                                     v149 = iconst.i64 0
;; @004e                               store notrap aligned v149, v120+8  ; v149 = 0
;;                                     v150 = uextend.i128 v149  ; v149 = 0
;;                                     v151 = iconst.i64 64
;;                                     v152 = ishl v150, v151  ; v151 = 64
;; @0040                               v8 = bor v152, v150
;; @0056                               jump block2(v8)
;;
;;                                 block2(v23: i128):
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
