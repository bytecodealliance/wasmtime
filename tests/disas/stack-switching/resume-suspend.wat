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
;; @003b                               v43 = iconst.i64 80
;; @003b                               v44 = iadd.i64 v10, v43  ; v43 = 80
;; @003b                               v45 = load.i64 notrap aligned v44
;; @003b                               v46 = iconst.i64 -24
;; @003b                               v47 = iadd v45, v46  ; v46 = -24
;; @003b                               v41 = uextend.i64 v20
;;                                     v59 = iconst.i64 0x0002_0000_0000
;;                                     v60 = bor v41, v59  ; v59 = 0x0002_0000_0000
;; @003b                               v48 = stack_switch v47, v47, v60
;; @003b                               v30 = iconst.i64 120
;; @003b                               v31 = iadd.i64 v8, v30  ; v30 = 120
;; @003b                               v51 = load.i64 notrap aligned v31+8
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
;; @0043                               v7 = iconst.i32 0
;; @0043                               v9 = call fn0(v0, v7)  ; v7 = 0
;; @0045                               trapz v9, user16
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
;; @004e                               trapz v138, user16
;; @004e                               v24 = load.i64 notrap aligned v138+72
;; @0045                               v15 = uextend.i128 v13
;; @0045                               v18 = bor v134, v15
;;                                     v140 = ushr v18, v130  ; v130 = 64
;; @004e                               v23 = ireduce.i64 v140
;; @004e                               v25 = icmp eq v24, v23
;; @004e                               trapz v25, user23
;; @004e                               v26 = iconst.i64 1
;; @004e                               v27 = iadd v24, v26  ; v26 = 1
;; @004e                               store notrap aligned v27, v138+72
;; @004e                               v28 = load.i64 notrap aligned v138+64
;; @004e                               v29 = load.i64 notrap aligned v0+8
;; @004e                               v30 = load.i64 notrap aligned v29+88
;; @004e                               v31 = load.i64 notrap aligned v29+96
;; @004e                               store notrap aligned v30, v28+48
;; @004e                               store notrap aligned v31, v28+56
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned v2, v138+64  ; v2 = 0
;; @004e                               v34 = load.i64 notrap aligned v0+8
;; @004e                               v33 = iconst.i64 2
;; @004e                               store notrap aligned v33, v34+88  ; v33 = 2
;; @004e                               store notrap aligned v138, v34+96
;; @004e                               v37 = iconst.i32 1
;; @004e                               v38 = iconst.i64 16
;; @004e                               v39 = iadd v138, v38  ; v38 = 16
;; @004e                               store notrap aligned v37, v39  ; v37 = 1
;; @004e                               v40 = iconst.i32 2
;; @004e                               v42 = iadd v31, v38  ; v38 = 16
;; @004e                               store notrap aligned v40, v42  ; v40 = 2
;; @004e                               v43 = load.i64 notrap aligned readonly v0+8
;; @004e                               v46 = load.i64 notrap aligned v43+72
;; @004e                               store notrap aligned v46, v31+8
;; @004e                               v47 = load.i64 notrap aligned v43+24
;; @004e                               store notrap aligned v47, v31
;; @004e                               v50 = load.i64 notrap aligned v138
;; @004e                               store notrap aligned v50, v43+24
;; @004e                               v51 = load.i64 notrap aligned v138+8
;; @004e                               store notrap aligned v51, v43+72
;; @004e                               v52 = iconst.i64 24
;; @004e                               v53 = iadd v31, v52  ; v52 = 24
;; @004e                               store notrap aligned v37, v53+4  ; v37 = 1
;; @004e                               v55 = stack_addr.i64 ss0
;; @004e                               store notrap aligned v55, v53+8
;; @004e                               v57 = iconst.i64 48
;; @004e                               v58 = iadd.i64 v0, v57  ; v57 = 48
;; @004e                               store notrap aligned v58, v55
;; @004e                               store notrap aligned v37, v53  ; v37 = 1
;; @004e                               store notrap aligned v37, v31+40  ; v37 = 1
;; @004e                               v64 = iconst.i64 80
;; @004e                               v65 = iadd v28, v64  ; v64 = 80
;; @004e                               v66 = load.i64 notrap aligned v65
;; @004e                               v67 = iconst.i64 -24
;; @004e                               v68 = iadd v66, v67  ; v67 = -24
;;                                     v142 = iconst.i64 0x0001_0000_0000
;; @004e                               v69 = stack_switch v68, v68, v142  ; v142 = 0x0001_0000_0000
;; @004e                               v70 = load.i64 notrap aligned v0+8
;; @004e                               v71 = load.i64 notrap aligned v70+88
;; @004e                               v72 = load.i64 notrap aligned v70+96
;; @004e                               store notrap aligned v30, v70+88
;; @004e                               store notrap aligned v31, v70+96
;; @004e                               store notrap aligned v37, v42  ; v37 = 1
;;                                     v145 = iconst.i32 0
;; @004e                               store notrap aligned v145, v53  ; v145 = 0
;; @004e                               store notrap aligned v145, v53+4  ; v145 = 0
;; @004e                               store notrap aligned v2, v53+8  ; v2 = 0
;; @004e                               store notrap aligned v2, v31+40  ; v2 = 0
;;                                     v125 = iconst.i64 32
;; @004e                               v80 = ushr v69, v125  ; v125 = 32
;; @004e                               brif v80, block5, block4
;;
;;                                 block5:
;; @004e                               v85 = load.i64 notrap aligned v43+72
;; @004e                               store notrap aligned v85, v72+8
;; @004e                               v88 = load.i64 notrap aligned v31
;; @004e                               store notrap aligned v88, v43+24
;; @004e                               v89 = load.i64 notrap aligned v31+8
;; @004e                               store notrap aligned v89, v43+72
;; @004e                               v91 = load.i64 notrap aligned v72+72
;; @004e                               jump block6
;;
;;                                 block7 cold:
;; @004e                               trap user12
;;
;;                                 block8:
;; @004e                               v96 = iconst.i64 120
;; @004e                               v97 = iadd.i64 v72, v96  ; v96 = 120
;; @004e                               v98 = load.i64 notrap aligned v97+8
;;                                     v153 = iconst.i32 0
;; @004e                               store notrap aligned v153, v97  ; v153 = 0
;; @004e                               v93 = uextend.i128 v91
;;                                     v154 = iconst.i64 64
;;                                     v155 = ishl v93, v154  ; v154 = 64
;; @004e                               v92 = uextend.i128 v72
;; @004e                               v95 = bor v155, v92
;; @004e                               jump block2(v95)
;;
;;                                 block6:
;; @004e                               v90 = ireduce.i32 v69
;; @004e                               br_table v90, block7, [block8]
;;
;;                                 block4:
;; @004e                               v102 = load.i64 notrap aligned v31
;; @004e                               store notrap aligned v102, v43+24
;; @004e                               v103 = load.i64 notrap aligned v31+8
;; @004e                               store notrap aligned v103, v43+72
;; @004e                               v106 = iconst.i32 4
;;                                     v146 = iconst.i64 16
;;                                     v147 = iadd.i64 v72, v146  ; v146 = 16
;; @004e                               store notrap aligned v106, v147  ; v106 = 4
;; @004e                               v109 = iconst.i64 104
;; @004e                               v110 = iadd.i64 v72, v109  ; v109 = 104
;; @004e                               v111 = load.i64 notrap aligned v110+8
;;                                     v148 = iconst.i32 0
;; @004e                               store notrap aligned v148, v110  ; v148 = 0
;; @004e                               store notrap aligned v148, v110+4  ; v148 = 0
;;                                     v149 = iconst.i64 0
;; @004e                               store notrap aligned v149, v110+8  ; v149 = 0
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
