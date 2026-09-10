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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 106 ""
;;     region3 = 225 ""
;;     region4 = 214 ""
;;     region5 = 13 ""
;;     region6 = 55 ""
;;     region7 = 153 ""
;;     region8 = 118 ""
;;     region9 = 82 ""
;;     region10 = 255 ""
;;     region11 = 211 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) -> i8 tail
;;     fn0 = colocated u805306368:44 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003c                               v3 = iconst.i32 10
;; @0044                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0044                               v31 = iconst.i64 144
;; @0044                               v34 = stack_addr.i64 ss0
;; @0044                               v37 = iconst.i64 0
;; @0044                               v47 = iconst.i64 96
;; @0044                               v50 = iconst.i64 -24
;;                                     v77 = iconst.i64 0x0002_0000_0000
;; @0044                               v43 = iconst.i64 32
;; @0044                               v55 = iconst.i64 5
;; @0040                               jump block2(v3)  ; v3 = 10
;;
;;                                 block2(v4: i32):
;; @0044                               v8 = load.i64 notrap aligned region2 v7+88
;; @0044                               v9 = load.i64 notrap aligned region2 v7+96
;; @0044                               v12 = iconst.i64 1
;; @0044                               v16 = iconst.i64 40
;; @003a                               v2 = iconst.i32 0
;; @0044                               jump block4(v8, v9, v4)
;;
;;                                 block4(v10: i64, v11: i64, v71: i32):
;;                                     v80 = iconst.i64 1
;;                                     v81 = icmp eq v10, v80  ; v80 = 1
;; @0044                               trapnz v81, user22
;; @0044                               jump block5
;;
;;                                 block5:
;; @0044                               v14 = load.i64 notrap aligned region3 v11+64
;; @0044                               v15 = load.i64 notrap aligned region3 v11+72
;;                                     v82 = iconst.i64 40
;;                                     v83 = iadd v15, v82  ; v82 = 40
;; @0044                               v18 = load.i64 notrap aligned region4 v83+8
;; @0044                               v19 = load.i32 notrap aligned region5 v15+56
;;                                     v84 = iconst.i32 0
;;                                     v74 = iconst.i32 3
;; @0044                               v5 = iconst.i64 48
;; @0044                               v6 = iadd.i64 v0, v5  ; v5 = 48
;; @0044                               v29 = iconst.i32 1
;; @0044                               jump block6(v84)  ; v84 = 0
;;
;;                                 block6(v21: i32):
;; @0044                               v22 = icmp ult v21, v19
;; @0044                               brif v22, block7, block4(v14, v15, v71)
;;
;;                                 block7:
;;                                     v85 = iconst.i32 3
;;                                     v86 = ishl.i32 v21, v85  ; v85 = 3
;; @0044                               v25 = uextend.i64 v86
;; @0044                               v26 = iadd.i64 v18, v25
;; @0044                               v27 = load.i64 notrap aligned region6 v26
;;                                     v87 = iadd.i64 v0, v5  ; v5 = 48
;;                                     v88 = icmp eq v27, v87
;;                                     v89 = iconst.i32 1
;;                                     v90 = iadd.i32 v21, v89  ; v89 = 1
;; @0044                               brif v88, block8, block6(v90)
;;
;;                                 block8:
;; @0044                               store.i64 notrap aligned region7 v11, v9+80
;;                                     v91 = iconst.i32 1
;;                                     v92 = iconst.i64 144
;;                                     v93 = iadd.i64 v9, v92  ; v92 = 144
;; @0044                               store notrap aligned region8 v91, v93+4  ; v91 = 1
;; @0044                               store.i64 notrap aligned region4 v34, v93+8
;; @0044                               store.i32 notrap aligned region6 v4, v34
;; @0044                               store notrap aligned region9 v91, v93  ; v91 = 1
;;                                     v94 = iconst.i32 3
;; @0044                               store notrap aligned region10 v94, v9+32  ; v94 = 3
;;                                     v95 = iconst.i64 0
;; @0044                               store notrap aligned region3 v95, v11+64  ; v95 = 0
;; @0044                               store notrap aligned region3 v95, v11+72  ; v95 = 0
;;                                     v96 = iconst.i64 96
;;                                     v97 = iadd.i64 v11, v96  ; v96 = 96
;; @0044                               v49 = load.i64 notrap aligned region11 v97
;;                                     v98 = iconst.i64 -24
;;                                     v99 = iadd v49, v98  ; v98 = -24
;; @0044                               v45 = uextend.i64 v21
;;                                     v100 = iconst.i64 0x0002_0000_0000
;;                                     v101 = bor v45, v100  ; v100 = 0x0002_0000_0000
;; @0044                               v52 = stack_switch v99, v99, v101
;;                                     v102 = iconst.i64 32
;;                                     v103 = ushr v52, v102  ; v102 = 32
;;                                     v104 = iconst.i64 5
;;                                     v105 = icmp eq v103, v104  ; v104 = 5
;; @0044                               brif v105, block10, block11
;;
;;                                 block10 cold:
;; @0044                               v59 = load.i64 notrap aligned region4 v93+8
;; @0044                               v60 = load.i32 notrap aligned region6 v59
;;                                     v110 = iconst.i32 0
;; @0044                               store notrap aligned region9 v110, v93  ; v110 = 0
;; @0044                               store notrap aligned region8 v110, v93+4  ; v110 = 0
;;                                     v111 = iconst.i64 0
;; @0044                               store notrap aligned region4 v111, v93+8  ; v111 = 0
;; @0044                               try_call fn0(v0, v60), sig0, block12, [ context v0 ]
;;
;;                                 block12:
;; @0044                               trap user12
;;
;;                                 block11:
;; @0044                               v66 = load.i64 notrap aligned region4 v93+8
;;                                     v106 = iconst.i32 0
;; @0044                               store notrap aligned region9 v106, v93  ; v106 = 0
;; @0044                               store notrap aligned region8 v106, v93+4  ; v106 = 0
;;                                     v107 = iconst.i64 0
;; @0044                               store notrap aligned region4 v107, v93+8  ; v107 = 0
;;                                     v108 = iconst.i32 1
;;                                     v109 = isub.i32 v71, v108  ; v108 = 1
;; @004d                               brif v109, block2(v109), block13
;;
;;                                 block13:
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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 206 ""
;;     region3 = 153 ""
;;     region4 = 106 ""
;;     region5 = 225 ""
;;     region6 = 255 ""
;;     region7 = 231 ""
;;     region8 = 243 ""
;;     region9 = 209 ""
;;     region10 = 23 ""
;;     region11 = 224 ""
;;     region12 = 13 ""
;;     region13 = 179 ""
;;     region14 = 118 ""
;;     region15 = 214 ""
;;     region16 = 55 ""
;;     region17 = 82 ""
;;     region18 = 211 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32, i32) -> i64 tail
;;     sig2 = (i64 vmctx) tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:42 sig1
;;     fn2 = colocated u805306368:41 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0056                               v2 = iconst.i32 0
;; @0056                               v3 = call fn0(v0, v2)  ; v2 = 0
;; @0058                               trapz v3, user16
;; @0058                               v7 = call fn1(v0, v3, v2, v2, v2)  ; v2 = 0, v2 = 0, v2 = 0
;; @0058                               v8 = load.i64 notrap aligned region2 v7+88
;; @0058                               v10 = uextend.i128 v8
;; @0058                               v11 = iconst.i64 64
;;                                     v139 = ishl v10, v11  ; v11 = 64
;; @0058                               v9 = uextend.i128 v7
;; @0058                               v14 = bor v139, v9
;; @0062                               v23 = iconst.i64 1
;; @0062                               v26 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0062                               v29 = iconst.i64 0
;; @0062                               v30 = iconst.i64 2
;; @0062                               v34 = iconst.i32 1
;; @0062                               v35 = iconst.i32 2
;; @0062                               v49 = iconst.i64 40
;; @0062                               v52 = stack_addr.i64 ss0
;; @0062                               v53 = iconst.i64 48
;; @0062                               v54 = iadd v0, v53  ; v53 = 48
;; @0062                               v61 = iconst.i64 96
;; @0062                               v64 = iconst.i64 -24
;;                                     v143 = iconst.i64 0x0001_0000_0000
;; @005c                               jump block2(v14)
;;
;;                                 block2(v15: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v16 = ireduce.i64 v15
;; @0062                               trapz v16, user16
;; @0062                               v21 = load.i64 notrap aligned region2 v16+88
;;                                     v146 = iconst.i64 64
;;                                     v147 = ushr.i128 v15, v146  ; v146 = 64
;; @0062                               v20 = ireduce.i64 v147
;; @0062                               v22 = icmp eq v21, v20
;; @0062                               trapz v22, user23
;;                                     v148 = iconst.i64 1
;;                                     v149 = iadd v21, v148  ; v148 = 1
;; @0062                               store notrap aligned region2 v149, v16+88
;; @0062                               v25 = load.i64 notrap aligned region3 v16+80
;; @0062                               v27 = load.i64 notrap aligned region4 v26+88
;; @0062                               v28 = load.i64 notrap aligned region4 v26+96
;; @0062                               store notrap aligned region5 v27, v25+64
;; @0062                               store notrap aligned region5 v28, v25+72
;;                                     v150 = iconst.i64 0
;; @0062                               store notrap aligned region3 v150, v16+80  ; v150 = 0
;;                                     v151 = iconst.i64 2
;; @0062                               store notrap aligned region4 v151, v26+88  ; v151 = 2
;; @0062                               store notrap aligned region4 v16, v26+96
;;                                     v152 = iconst.i32 1
;; @0062                               store notrap aligned region6 v152, v16+32  ; v152 = 1
;;                                     v153 = iconst.i32 2
;; @0062                               store notrap aligned region6 v153, v28+32  ; v153 = 2
;; @0062                               v39 = load.i64 notrap aligned region7 v26+72
;; @0062                               v40 = load.i64 notrap aligned region8 v26+64
;; @0062                               v41 = load.i64 notrap aligned region9 v26+80
;; @0062                               store notrap aligned region10 v39, v28+8
;; @0062                               store notrap aligned region11 v40, v28+16
;; @0062                               store notrap aligned region12 v41, v28+24
;; @0062                               v42 = load.i64 notrap aligned region1 v26+24
;; @0062                               store notrap aligned region13 v42, v28
;; @0062                               v45 = load.i64 notrap aligned region13 v16
;; @0062                               store notrap aligned region1 v45, v26+24
;; @0062                               v46 = load.i64 notrap aligned region10 v16+8
;; @0062                               store notrap aligned region7 v46, v26+72
;; @0062                               v47 = load.i64 notrap aligned region11 v16+16
;; @0062                               store notrap aligned region8 v47, v26+64
;; @0062                               v48 = load.i64 notrap aligned region12 v16+24
;; @0062                               store notrap aligned region9 v48, v26+80
;;                                     v154 = iconst.i64 40
;;                                     v155 = iadd v28, v154  ; v154 = 40
;; @0062                               store notrap aligned region14 v152, v155+4  ; v152 = 1
;; @0062                               store.i64 notrap aligned region15 v52, v155+8
;;                                     v156 = iadd.i64 v0, v53  ; v53 = 48
;; @0062                               store notrap aligned region16 v156, v52
;; @0062                               store notrap aligned region17 v152, v155  ; v152 = 1
;; @0062                               store notrap aligned region12 v152, v28+56  ; v152 = 1
;;                                     v157 = iconst.i64 96
;;                                     v158 = iadd v25, v157  ; v157 = 96
;; @0062                               v63 = load.i64 notrap aligned region18 v158
;;                                     v159 = iconst.i64 -24
;;                                     v160 = iadd v63, v159  ; v159 = -24
;;                                     v161 = iconst.i64 0x0001_0000_0000
;; @0062                               v66 = stack_switch v160, v160, v161  ; v161 = 0x0001_0000_0000
;; @0062                               v68 = load.i64 notrap aligned region4 v26+88
;; @0062                               v69 = load.i64 notrap aligned region4 v26+96
;; @0062                               store notrap aligned region4 v27, v26+88
;; @0062                               store notrap aligned region4 v28, v26+96
;; @0062                               store notrap aligned region6 v152, v28+32  ; v152 = 1
;;                                     v162 = iconst.i32 0
;; @0062                               store notrap aligned region17 v162, v155  ; v162 = 0
;; @0062                               store notrap aligned region14 v162, v155+4  ; v162 = 0
;; @0062                               store notrap aligned region15 v150, v155+8  ; v150 = 0
;; @0062                               store notrap aligned region12 v150, v28+56  ; v150 = 0
;; @0062                               brif v66, block9, block6
;;
;;                                 block9:
;; @0062                               v59 = iconst.i64 32
;; @0062                               v76 = ushr.i64 v66, v59  ; v59 = 32
;; @0062                               v77 = iconst.i64 4
;; @0062                               v78 = icmp eq v76, v77  ; v77 = 4
;; @0062                               brif v78, block8, block7
;;
;;                                 block8 cold:
;; @0062                               v81 = iconst.i32 5
;; @0062                               store notrap aligned region6 v81, v69+32  ; v81 = 5
;; @0062                               v84 = load.i64 notrap aligned region13 v28
;; @0062                               store notrap aligned region1 v84, v26+24
;; @0062                               v85 = load.i64 notrap aligned region10 v28+8
;; @0062                               store notrap aligned region7 v85, v26+72
;; @0062                               v86 = load.i64 notrap aligned region11 v28+16
;; @0062                               store notrap aligned region8 v86, v26+64
;; @0062                               v87 = load.i64 notrap aligned region12 v28+24
;; @0062                               store notrap aligned region9 v87, v26+80
;;                                     v168 = iconst.i32 0
;;                                     v169 = iconst.i64 120
;;                                     v170 = iadd.i64 v69, v169  ; v169 = 120
;; @0062                               store notrap aligned region17 v168, v170  ; v168 = 0
;; @0062                               store notrap aligned region14 v168, v170+4  ; v168 = 0
;;                                     v171 = iconst.i64 0
;; @0062                               store notrap aligned region15 v171, v170+8  ; v171 = 0
;;                                     v172 = iconst.i64 144
;;                                     v173 = iadd.i64 v69, v172  ; v172 = 144
;; @0062                               store notrap aligned region17 v168, v173  ; v168 = 0
;; @0062                               store notrap aligned region14 v168, v173+4  ; v168 = 0
;; @0062                               store notrap aligned region15 v171, v173+8  ; v171 = 0
;; @0062                               try_call fn2(v0), sig2, block11, [ context v0 ]
;;
;;                                 block11:
;; @0062                               trap user12
;;
;;                                 block7:
;; @0062                               v102 = load.i64 notrap aligned region7 v26+72
;; @0062                               v103 = load.i64 notrap aligned region8 v26+64
;; @0062                               v104 = load.i64 notrap aligned region9 v26+80
;; @0062                               store notrap aligned region10 v102, v69+8
;; @0062                               store notrap aligned region11 v103, v69+16
;; @0062                               store notrap aligned region12 v104, v69+24
;; @0062                               v107 = load.i64 notrap aligned region13 v28
;; @0062                               store notrap aligned region1 v107, v26+24
;; @0062                               v108 = load.i64 notrap aligned region10 v28+8
;; @0062                               store notrap aligned region7 v108, v26+72
;; @0062                               v109 = load.i64 notrap aligned region11 v28+16
;; @0062                               store notrap aligned region8 v109, v26+64
;; @0062                               v110 = load.i64 notrap aligned region12 v28+24
;; @0062                               store notrap aligned region9 v110, v26+80
;; @0062                               v112 = load.i64 notrap aligned region2 v69+88
;; @0062                               jump block10
;;
;;                                 block12 cold:
;; @0062                               trap user12
;;
;;                                 block13:
;; @0062                               v119 = iconst.i64 144
;; @0062                               v120 = iadd.i64 v69, v119  ; v119 = 144
;; @0062                               v121 = load.i64 notrap aligned region15 v120+8
;; @0062                               v122 = load.i32 notrap aligned region16 v121
;;                                     v165 = iconst.i32 0
;; @0062                               store notrap aligned region17 v165, v120  ; v165 = 0
;; @0062                               jump block4
;;
;;                                 block10:
;; @0062                               v111 = ireduce.i32 v66
;; @0062                               br_table v111, block12, [block13]
;;
;;                                 block6:
;; @0062                               v126 = load.i64 notrap aligned region13 v28
;; @0062                               store notrap aligned region1 v126, v26+24
;; @0062                               v127 = load.i64 notrap aligned region10 v28+8
;; @0062                               store notrap aligned region7 v127, v26+72
;; @0062                               v128 = load.i64 notrap aligned region11 v28+16
;; @0062                               store notrap aligned region8 v128, v26+64
;; @0062                               v129 = load.i64 notrap aligned region12 v28+24
;; @0062                               store notrap aligned region9 v129, v26+80
;; @0062                               v132 = iconst.i32 4
;; @0062                               store notrap aligned region6 v132, v69+32  ; v132 = 4
;; @0062                               v133 = iconst.i64 120
;; @0062                               v134 = iadd.i64 v69, v133  ; v133 = 120
;; @0062                               v135 = load.i64 notrap aligned region15 v134+8
;;                                     v163 = iconst.i32 0
;; @0062                               store notrap aligned region17 v163, v134  ; v163 = 0
;; @0062                               store notrap aligned region14 v163, v134+4  ; v163 = 0
;;                                     v164 = iconst.i64 0
;; @0062                               store notrap aligned region15 v164, v134+8  ; v164 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v114 = uextend.i128 v112
;;                                     v166 = iconst.i64 64
;;                                     v167 = ishl v114, v166  ; v166 = 64
;; @0062                               v113 = uextend.i128 v69
;; @0062                               v118 = bor v167, v113
;; @006d                               jump block2(v118)
;; }
