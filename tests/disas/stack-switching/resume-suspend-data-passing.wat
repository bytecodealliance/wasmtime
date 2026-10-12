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
;;     ss0 = explicit_slot 16, align = 16
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 106 ""
;;     region3 = 206 ""
;;     region4 = 214 ""
;;     region5 = 13 ""
;;     region6 = 55 ""
;;     region7 = 26 ""
;;     region8 = 118 ""
;;     region9 = 82 ""
;;     region10 = 255 ""
;;     region11 = 221 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) -> i8 tail
;;     fn0 = colocated u805306368:43 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003c                               v3 = iconst.i32 10
;; @0044                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0044                               v31 = iconst.i64 168
;; @0044                               v34 = stack_addr.i64 ss0
;; @0044                               v37 = iconst.i64 0
;; @0044                               v47 = iconst.i64 120
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
;; @0044                               v14 = load.i64 notrap aligned region3 v11+88
;; @0044                               v15 = load.i64 notrap aligned region3 v11+96
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
;; @0044                               store.i64 notrap aligned region7 v11, v9+112
;;                                     v91 = iconst.i32 1
;;                                     v92 = iconst.i64 168
;;                                     v93 = iadd.i64 v9, v92  ; v92 = 168
;; @0044                               store notrap aligned region8 v91, v93+4  ; v91 = 1
;; @0044                               store.i64 notrap aligned region4 v34, v93+8
;; @0044                               store.i32 notrap aligned region6 v4, v34
;; @0044                               store notrap aligned region9 v91, v93  ; v91 = 1
;;                                     v94 = iconst.i32 3
;; @0044                               store notrap aligned region10 v94, v9+32  ; v94 = 3
;;                                     v95 = iconst.i64 0
;; @0044                               store notrap aligned region3 v95, v11+88  ; v95 = 0
;; @0044                               store notrap aligned region3 v95, v11+96  ; v95 = 0
;;                                     v96 = iconst.i64 120
;;                                     v97 = iadd.i64 v11, v96  ; v96 = 120
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
;;     ss0 = explicit_slot 8, align = 8
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 130 ""
;;     region3 = 6 ""
;;     region4 = 134 ""
;;     region5 = 116 ""
;;     region6 = 60 ""
;;     region7 = 106 ""
;;     region8 = 28 ""
;;     region9 = 26 ""
;;     region10 = 206 ""
;;     region11 = 255 ""
;;     region12 = 231 ""
;;     region13 = 243 ""
;;     region14 = 209 ""
;;     region15 = 23 ""
;;     region16 = 224 ""
;;     region17 = 13 ""
;;     region18 = 179 ""
;;     region19 = 118 ""
;;     region20 = 214 ""
;;     region21 = 55 ""
;;     region22 = 82 ""
;;     region23 = 221 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64, i64, i64) -> i8 system_v
;;     sig2 = (i64 vmctx, i64, i32, i32, i32) -> i64 tail
;;     sig3 = (i64 vmctx) tail
;;     fn0 = colocated u0:0 sig0
;;     fn1 = colocated u268435456:0 sig1
;;     fn2 = colocated u805306368:41 sig2
;;     fn3 = colocated u805306368:40 sig3
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0056                               v3 = func_addr.i64 fn1
;; @0056                               store notrap aligned region4 v3, v0+84
;; @0056                               v2 = func_addr.i64 fn0
;; @0056                               store notrap aligned region5 v2, v0+92
;; @0056                               store notrap aligned region6 v0, v0+108
;; @0056                               v4 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0056                               v5 = load.i32 notrap aligned readonly can_move region3 v4
;; @0056                               store notrap aligned region7 v5, v0+100
;; @0056                               v6 = iconst.i64 84
;; @0056                               v7 = iadd v0, v6  ; v6 = 84
;; @0058                               trapz v7, user16
;; @0058                               v8 = iconst.i32 0
;; @0058                               v11 = call fn2(v0, v7, v8, v8, v8)  ; v8 = 0, v8 = 0, v8 = 0
;; @0058                               v12 = load.i64 notrap aligned region8 v11+104
;; @0058                               v14 = uextend.i128 v12
;; @0058                               v15 = iconst.i64 64
;;                                     v143 = ishl v14, v15  ; v15 = 64
;; @0058                               v13 = uextend.i128 v11
;; @0058                               v18 = bor v143, v13
;; @0062                               v27 = iconst.i64 1
;; @0062                               v30 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0062                               v33 = iconst.i64 0
;; @0062                               v34 = iconst.i64 2
;; @0062                               v38 = iconst.i32 1
;; @0062                               v39 = iconst.i32 2
;; @0062                               v53 = iconst.i64 40
;; @0062                               v56 = stack_addr.i64 ss0
;; @0062                               v57 = iconst.i64 48
;; @0062                               v58 = iadd v0, v57  ; v57 = 48
;; @0062                               v65 = iconst.i64 120
;; @0062                               v68 = iconst.i64 -24
;;                                     v147 = iconst.i64 0x0001_0000_0000
;; @0062                               v63 = iconst.i64 32
;; @0062                               v81 = iconst.i64 4
;; @0062                               v123 = iconst.i64 168
;; @005c                               jump block2(v18)
;;
;;                                 block2(v19: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v20 = ireduce.i64 v19
;; @0062                               trapz v20, user16
;; @0062                               v25 = load.i64 notrap aligned region8 v20+104
;;                                     v150 = iconst.i64 64
;;                                     v151 = ushr.i128 v19, v150  ; v150 = 64
;; @0062                               v24 = ireduce.i64 v151
;; @0062                               v26 = icmp eq v25, v24
;; @0062                               trapz v26, user23
;;                                     v152 = iconst.i64 1
;;                                     v153 = iadd v25, v152  ; v152 = 1
;; @0062                               store notrap aligned region8 v153, v20+104
;; @0062                               v29 = load.i64 notrap aligned region9 v20+112
;; @0062                               v31 = load.i64 notrap aligned region7 v30+88
;; @0062                               v32 = load.i64 notrap aligned region7 v30+96
;; @0062                               store notrap aligned region10 v31, v29+88
;; @0062                               store notrap aligned region10 v32, v29+96
;;                                     v154 = iconst.i64 0
;; @0062                               store notrap aligned region9 v154, v20+112  ; v154 = 0
;;                                     v155 = iconst.i64 2
;; @0062                               store notrap aligned region7 v155, v30+88  ; v155 = 2
;; @0062                               store notrap aligned region7 v20, v30+96
;;                                     v156 = iconst.i32 1
;; @0062                               store notrap aligned region11 v156, v20+32  ; v156 = 1
;;                                     v157 = iconst.i32 2
;; @0062                               store notrap aligned region11 v157, v32+32  ; v157 = 2
;; @0062                               v43 = load.i64 notrap aligned region12 v30+72
;; @0062                               v44 = load.i64 notrap aligned region13 v30+64
;; @0062                               v45 = load.i64 notrap aligned region14 v30+80
;; @0062                               store notrap aligned region15 v43, v32+8
;; @0062                               store notrap aligned region16 v44, v32+16
;; @0062                               store notrap aligned region17 v45, v32+24
;; @0062                               v46 = load.i64 notrap aligned region1 v30+24
;; @0062                               store notrap aligned region18 v46, v32
;; @0062                               v49 = load.i64 notrap aligned region18 v20
;; @0062                               store notrap aligned region1 v49, v30+24
;; @0062                               v50 = load.i64 notrap aligned region15 v20+8
;; @0062                               store notrap aligned region12 v50, v30+72
;; @0062                               v51 = load.i64 notrap aligned region16 v20+16
;; @0062                               store notrap aligned region13 v51, v30+64
;; @0062                               v52 = load.i64 notrap aligned region17 v20+24
;; @0062                               store notrap aligned region14 v52, v30+80
;;                                     v158 = iconst.i64 40
;;                                     v159 = iadd v32, v158  ; v158 = 40
;; @0062                               store notrap aligned region19 v156, v159+4  ; v156 = 1
;; @0062                               store.i64 notrap aligned region20 v56, v159+8
;;                                     v160 = iadd.i64 v0, v57  ; v57 = 48
;; @0062                               store notrap aligned region21 v160, v56
;; @0062                               store notrap aligned region22 v156, v159  ; v156 = 1
;; @0062                               store notrap aligned region17 v156, v32+56  ; v156 = 1
;;                                     v161 = iconst.i64 120
;;                                     v162 = iadd v29, v161  ; v161 = 120
;; @0062                               v67 = load.i64 notrap aligned region23 v162
;;                                     v163 = iconst.i64 -24
;;                                     v164 = iadd v67, v163  ; v163 = -24
;;                                     v165 = iconst.i64 0x0001_0000_0000
;; @0062                               v70 = stack_switch v164, v164, v165  ; v165 = 0x0001_0000_0000
;; @0062                               v72 = load.i64 notrap aligned region7 v30+88
;; @0062                               v73 = load.i64 notrap aligned region7 v30+96
;; @0062                               store notrap aligned region7 v31, v30+88
;; @0062                               store notrap aligned region7 v32, v30+96
;; @0062                               store notrap aligned region11 v156, v32+32  ; v156 = 1
;;                                     v166 = iconst.i32 0
;; @0062                               store notrap aligned region22 v166, v159  ; v166 = 0
;; @0062                               store notrap aligned region19 v166, v159+4  ; v166 = 0
;; @0062                               store notrap aligned region20 v154, v159+8  ; v154 = 0
;; @0062                               store notrap aligned region17 v154, v32+56  ; v154 = 0
;; @0062                               brif v70, block9, block6
;;
;;                                 block9:
;;                                     v169 = iconst.i64 32
;;                                     v170 = ushr.i64 v70, v169  ; v169 = 32
;;                                     v171 = iconst.i64 4
;;                                     v172 = icmp eq v170, v171  ; v171 = 4
;; @0062                               brif v172, block8, block7
;;
;;                                 block8 cold:
;; @0062                               v85 = iconst.i32 5
;; @0062                               store notrap aligned region11 v85, v73+32  ; v85 = 5
;; @0062                               v88 = load.i64 notrap aligned region18 v32
;; @0062                               store notrap aligned region1 v88, v30+24
;; @0062                               v89 = load.i64 notrap aligned region15 v32+8
;; @0062                               store notrap aligned region12 v89, v30+72
;; @0062                               v90 = load.i64 notrap aligned region16 v32+16
;; @0062                               store notrap aligned region13 v90, v30+64
;; @0062                               v91 = load.i64 notrap aligned region17 v32+24
;; @0062                               store notrap aligned region14 v91, v30+80
;;                                     v178 = iconst.i32 0
;;                                     v179 = iconst.i64 144
;;                                     v180 = iadd.i64 v73, v179  ; v179 = 144
;; @0062                               store notrap aligned region22 v178, v180  ; v178 = 0
;; @0062                               store notrap aligned region19 v178, v180+4  ; v178 = 0
;;                                     v181 = iconst.i64 0
;; @0062                               store notrap aligned region20 v181, v180+8  ; v181 = 0
;;                                     v182 = iconst.i64 168
;;                                     v183 = iadd.i64 v73, v182  ; v182 = 168
;; @0062                               store notrap aligned region22 v178, v183  ; v178 = 0
;; @0062                               store notrap aligned region19 v178, v183+4  ; v178 = 0
;; @0062                               store notrap aligned region20 v181, v183+8  ; v181 = 0
;; @0062                               try_call fn3(v0), sig3, block11, [ context v0 ]
;;
;;                                 block11:
;; @0062                               trap user12
;;
;;                                 block7:
;; @0062                               v106 = load.i64 notrap aligned region12 v30+72
;; @0062                               v107 = load.i64 notrap aligned region13 v30+64
;; @0062                               v108 = load.i64 notrap aligned region14 v30+80
;; @0062                               store notrap aligned region15 v106, v73+8
;; @0062                               store notrap aligned region16 v107, v73+16
;; @0062                               store notrap aligned region17 v108, v73+24
;; @0062                               v111 = load.i64 notrap aligned region18 v32
;; @0062                               store notrap aligned region1 v111, v30+24
;; @0062                               v112 = load.i64 notrap aligned region15 v32+8
;; @0062                               store notrap aligned region12 v112, v30+72
;; @0062                               v113 = load.i64 notrap aligned region16 v32+16
;; @0062                               store notrap aligned region13 v113, v30+64
;; @0062                               v114 = load.i64 notrap aligned region17 v32+24
;; @0062                               store notrap aligned region14 v114, v30+80
;; @0062                               v116 = load.i64 notrap aligned region8 v73+104
;; @0062                               jump block10
;;
;;                                 block12 cold:
;; @0062                               trap user12
;;
;;                                 block13:
;;                                     v173 = iconst.i64 168
;;                                     v174 = iadd.i64 v73, v173  ; v173 = 168
;; @0062                               v125 = load.i64 notrap aligned region20 v174+8
;; @0062                               v126 = load.i32 notrap aligned region21 v125
;;                                     v175 = iconst.i32 0
;; @0062                               store notrap aligned region22 v175, v174  ; v175 = 0
;; @0062                               jump block4
;;
;;                                 block10:
;; @0062                               v115 = ireduce.i32 v70
;; @0062                               br_table v115, block12, [block13]
;;
;;                                 block6:
;; @0062                               v130 = load.i64 notrap aligned region18 v32
;; @0062                               store notrap aligned region1 v130, v30+24
;; @0062                               v131 = load.i64 notrap aligned region15 v32+8
;; @0062                               store notrap aligned region12 v131, v30+72
;; @0062                               v132 = load.i64 notrap aligned region16 v32+16
;; @0062                               store notrap aligned region13 v132, v30+64
;; @0062                               v133 = load.i64 notrap aligned region17 v32+24
;; @0062                               store notrap aligned region14 v133, v30+80
;; @0062                               v136 = iconst.i32 4
;; @0062                               store notrap aligned region11 v136, v73+32  ; v136 = 4
;; @0062                               v137 = iconst.i64 144
;; @0062                               v138 = iadd.i64 v73, v137  ; v137 = 144
;; @0062                               v139 = load.i64 notrap aligned region20 v138+8
;;                                     v167 = iconst.i32 0
;; @0062                               store notrap aligned region22 v167, v138  ; v167 = 0
;; @0062                               store notrap aligned region19 v167, v138+4  ; v167 = 0
;;                                     v168 = iconst.i64 0
;; @0062                               store notrap aligned region20 v168, v138+8  ; v168 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v118 = uextend.i128 v116
;;                                     v176 = iconst.i64 64
;;                                     v177 = ishl v118, v176  ; v176 = 64
;; @0062                               v117 = uextend.i128 v73
;; @0062                               v122 = bor v177, v117
;; @006d                               jump block2(v122)
;; }
