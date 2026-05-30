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
;; @004e                               v28 = load.i64 notrap aligned v138+72
;; @0045                               v17 = uextend.i128 v15
;; @0045                               v22 = bor v134, v17
;;                                     v140 = ushr v22, v5  ; v5 = 64
;; @004e                               v27 = ireduce.i64 v140
;; @004e                               v29 = icmp eq v28, v27
;; @004e                               trapz v29, user23
;; @004e                               v30 = iconst.i64 1
;; @004e                               v31 = iadd v28, v30  ; v30 = 1
;; @004e                               store notrap aligned v31, v138+72
;; @004e                               v32 = load.i64 notrap aligned v138+64
;; @004e                               v33 = load.i64 notrap aligned v0+8
;; @004e                               v34 = load.i64 notrap aligned v33+88
;; @004e                               v35 = load.i64 notrap aligned v33+96
;; @004e                               store notrap aligned v34, v32+48
;; @004e                               store notrap aligned v35, v32+56
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned v2, v138+64  ; v2 = 0
;; @004e                               v38 = load.i64 notrap aligned v0+8
;; @004e                               v37 = iconst.i64 2
;; @004e                               store notrap aligned v37, v38+88  ; v37 = 2
;; @004e                               store notrap aligned v138, v38+96
;; @004e                               v41 = iconst.i32 1
;; @004e                               v42 = iconst.i64 16
;; @004e                               v43 = iadd v138, v42  ; v42 = 16
;; @004e                               store notrap aligned v41, v43  ; v41 = 1
;; @004e                               v44 = iconst.i32 2
;; @004e                               v46 = iadd v35, v42  ; v42 = 16
;; @004e                               store notrap aligned v44, v46  ; v44 = 2
;; @004e                               v47 = load.i64 notrap aligned readonly v0+8
;; @004e                               v50 = load.i64 notrap aligned v47+72
;; @004e                               store notrap aligned v50, v35+8
;; @004e                               v51 = load.i64 notrap aligned v47+24
;; @004e                               store notrap aligned v51, v35
;; @004e                               v54 = load.i64 notrap aligned v138
;; @004e                               store notrap aligned v54, v47+24
;; @004e                               v55 = load.i64 notrap aligned v138+8
;; @004e                               store notrap aligned v55, v47+72
;; @004e                               v56 = iconst.i64 24
;; @004e                               v57 = iadd v35, v56  ; v56 = 24
;; @004e                               store notrap aligned v41, v57+4  ; v41 = 1
;; @004e                               v59 = stack_addr.i64 ss0
;; @004e                               store notrap aligned v59, v57+8
;; @004e                               v61 = iconst.i64 48
;; @004e                               v62 = iadd.i64 v0, v61  ; v61 = 48
;; @004e                               store notrap aligned v62, v59
;; @004e                               store notrap aligned v41, v57  ; v41 = 1
;; @004e                               store notrap aligned v41, v35+40  ; v41 = 1
;; @004e                               v69 = iconst.i64 80
;; @004e                               v70 = iadd v32, v69  ; v69 = 80
;; @004e                               v71 = load.i64 notrap aligned v70
;; @004e                               v72 = iconst.i64 -24
;; @004e                               v73 = iadd v71, v72  ; v72 = -24
;;                                     v142 = iconst.i64 0x0001_0000_0000
;; @004e                               v74 = stack_switch v73, v73, v142  ; v142 = 0x0001_0000_0000
;; @004e                               v75 = load.i64 notrap aligned v0+8
;; @004e                               v76 = load.i64 notrap aligned v75+88
;; @004e                               v77 = load.i64 notrap aligned v75+96
;; @004e                               store notrap aligned v34, v75+88
;; @004e                               store notrap aligned v35, v75+96
;; @004e                               store notrap aligned v41, v46  ; v41 = 1
;;                                     v145 = iconst.i32 0
;; @004e                               store notrap aligned v145, v57  ; v145 = 0
;; @004e                               store notrap aligned v145, v57+4  ; v145 = 0
;; @004e                               store notrap aligned v2, v57+8  ; v2 = 0
;; @004e                               store notrap aligned v2, v35+40  ; v2 = 0
;; @004e                               v67 = iconst.i64 32
;; @004e                               v85 = ushr v74, v67  ; v67 = 32
;; @004e                               brif v85, block5, block4
;;
;;                                 block5:
;; @004e                               v90 = load.i64 notrap aligned v47+72
;; @004e                               store notrap aligned v90, v77+8
;; @004e                               v93 = load.i64 notrap aligned v35
;; @004e                               store notrap aligned v93, v47+24
;; @004e                               v94 = load.i64 notrap aligned v35+8
;; @004e                               store notrap aligned v94, v47+72
;; @004e                               v96 = load.i64 notrap aligned v77+72
;; @004e                               jump block6
;;
;;                                 block7 cold:
;; @004e                               trap user12
;;
;;                                 block8:
;; @004e                               v103 = iconst.i64 120
;; @004e                               v104 = iadd.i64 v77, v103  ; v103 = 120
;; @004e                               v105 = load.i64 notrap aligned v104+8
;;                                     v153 = iconst.i32 0
;; @004e                               store notrap aligned v153, v104  ; v153 = 0
;; @004e                               v98 = uextend.i128 v96
;;                                     v154 = iconst.i64 64
;;                                     v155 = ishl v98, v154  ; v154 = 64
;; @004e                               v97 = uextend.i128 v77
;; @004e                               v102 = bor v155, v97
;; @004e                               jump block2(v102)
;;
;;                                 block6:
;; @004e                               v95 = ireduce.i32 v74
;; @004e                               br_table v95, block7, [block8]
;;
;;                                 block4:
;; @004e                               v109 = load.i64 notrap aligned v35
;; @004e                               store notrap aligned v109, v47+24
;; @004e                               v110 = load.i64 notrap aligned v35+8
;; @004e                               store notrap aligned v110, v47+72
;; @004e                               v113 = iconst.i32 4
;;                                     v146 = iconst.i64 16
;;                                     v147 = iadd.i64 v77, v146  ; v146 = 16
;; @004e                               store notrap aligned v113, v147  ; v113 = 4
;; @004e                               v116 = iconst.i64 104
;; @004e                               v117 = iadd.i64 v77, v116  ; v116 = 104
;; @004e                               v118 = load.i64 notrap aligned v117+8
;;                                     v148 = iconst.i32 0
;; @004e                               store notrap aligned v148, v117  ; v148 = 0
;; @004e                               store notrap aligned v148, v117+4  ; v148 = 0
;;                                     v149 = iconst.i64 0
;; @004e                               store notrap aligned v149, v117+8  ; v149 = 0
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
