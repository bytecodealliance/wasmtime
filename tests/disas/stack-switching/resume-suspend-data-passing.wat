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
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 124 "VMStoreContext+0x58"
;;     region3 = 66 "VMContRef+0x40"
;;     region4 = 229 "VMHostArray+0x8"
;;     region5 = 23 "VMCommonStackInformation+0x38"
;;     region6 = 212 "ContinuationStackMemory+0x0"
;;     region7 = 182 "VMContRef+0x50"
;;     region8 = 20 "VMHostArray+0x4"
;;     region9 = 126 "VMHostArray+0x0"
;;     region10 = 176 "VMCommonStackInformation+0x20"
;;     region11 = 151 "VMContRef+0x60"
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
;; @0044                               v31 = iconst.i64 136
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
;;                                     v92 = iconst.i64 136
;;                                     v93 = iadd.i64 v9, v92  ; v92 = 136
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
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 84 "VMContRef+0x58"
;;     region3 = 182 "VMContRef+0x50"
;;     region4 = 124 "VMStoreContext+0x58"
;;     region5 = 66 "VMContRef+0x40"
;;     region6 = 176 "VMCommonStackInformation+0x20"
;;     region7 = 108 "VMStoreContext+0x48"
;;     region8 = 38 "VMStoreContext+0x40"
;;     region9 = 203 "VMStoreContext+0x50"
;;     region10 = 89 "VMStackLimits+0x8"
;;     region11 = 150 "VMStackLimits+0x10"
;;     region12 = 232 "VMStackLimits+0x18"
;;     region13 = 180 "VMStackLimits+0x0"
;;     region14 = 20 "VMHostArray+0x4"
;;     region15 = 229 "VMHostArray+0x8"
;;     region16 = 212 "ContinuationStackMemory+0x0"
;;     region17 = 126 "VMHostArray+0x0"
;;     region18 = 23 "VMCommonStackInformation+0x38"
;;     region19 = 151 "VMContRef+0x60"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
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
;; @0058                               v6 = call fn1(v0, v3, v2, v2)  ; v2 = 0, v2 = 0
;; @0058                               v7 = load.i64 notrap aligned region2 v6+88
;; @0058                               v9 = uextend.i128 v7
;; @0058                               v10 = iconst.i64 64
;;                                     v138 = ishl v9, v10  ; v10 = 64
;; @0058                               v8 = uextend.i128 v6
;; @0058                               v13 = bor v138, v8
;; @0062                               v22 = iconst.i64 1
;; @0062                               v25 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0062                               v28 = iconst.i64 0
;; @0062                               v29 = iconst.i64 2
;; @0062                               v33 = iconst.i32 1
;; @0062                               v34 = iconst.i32 2
;; @0062                               v48 = iconst.i64 40
;; @0062                               v51 = stack_addr.i64 ss0
;; @0062                               v52 = iconst.i64 48
;; @0062                               v53 = iadd v0, v52  ; v52 = 48
;; @0062                               v60 = iconst.i64 96
;; @0062                               v63 = iconst.i64 -24
;;                                     v142 = iconst.i64 0x0001_0000_0000
;; @005c                               jump block2(v13)
;;
;;                                 block2(v14: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v15 = ireduce.i64 v14
;; @0062                               trapz v15, user16
;; @0062                               v20 = load.i64 notrap aligned region2 v15+88
;;                                     v145 = iconst.i64 64
;;                                     v146 = ushr.i128 v14, v145  ; v145 = 64
;; @0062                               v19 = ireduce.i64 v146
;; @0062                               v21 = icmp eq v20, v19
;; @0062                               trapz v21, user23
;;                                     v147 = iconst.i64 1
;;                                     v148 = iadd v20, v147  ; v147 = 1
;; @0062                               store notrap aligned region2 v148, v15+88
;; @0062                               v24 = load.i64 notrap aligned region3 v15+80
;; @0062                               v26 = load.i64 notrap aligned region4 v25+88
;; @0062                               v27 = load.i64 notrap aligned region4 v25+96
;; @0062                               store notrap aligned region5 v26, v24+64
;; @0062                               store notrap aligned region5 v27, v24+72
;;                                     v149 = iconst.i64 0
;; @0062                               store notrap aligned region3 v149, v15+80  ; v149 = 0
;;                                     v150 = iconst.i64 2
;; @0062                               store notrap aligned region4 v150, v25+88  ; v150 = 2
;; @0062                               store notrap aligned region4 v15, v25+96
;;                                     v151 = iconst.i32 1
;; @0062                               store notrap aligned region6 v151, v15+32  ; v151 = 1
;;                                     v152 = iconst.i32 2
;; @0062                               store notrap aligned region6 v152, v27+32  ; v152 = 2
;; @0062                               v38 = load.i64 notrap aligned region7 v25+72
;; @0062                               v39 = load.i64 notrap aligned region8 v25+64
;; @0062                               v40 = load.i64 notrap aligned region9 v25+80
;; @0062                               store notrap aligned region10 v38, v27+8
;; @0062                               store notrap aligned region11 v39, v27+16
;; @0062                               store notrap aligned region12 v40, v27+24
;; @0062                               v41 = load.i64 notrap aligned region1 v25+24
;; @0062                               store notrap aligned region13 v41, v27
;; @0062                               v44 = load.i64 notrap aligned region13 v15
;; @0062                               store notrap aligned region1 v44, v25+24
;; @0062                               v45 = load.i64 notrap aligned region10 v15+8
;; @0062                               store notrap aligned region7 v45, v25+72
;; @0062                               v46 = load.i64 notrap aligned region11 v15+16
;; @0062                               store notrap aligned region8 v46, v25+64
;; @0062                               v47 = load.i64 notrap aligned region12 v15+24
;; @0062                               store notrap aligned region9 v47, v25+80
;;                                     v153 = iconst.i64 40
;;                                     v154 = iadd v27, v153  ; v153 = 40
;; @0062                               store notrap aligned region14 v151, v154+4  ; v151 = 1
;; @0062                               store.i64 notrap aligned region15 v51, v154+8
;;                                     v155 = iadd.i64 v0, v52  ; v52 = 48
;; @0062                               store notrap aligned region16 v155, v51
;; @0062                               store notrap aligned region17 v151, v154  ; v151 = 1
;; @0062                               store notrap aligned region18 v151, v27+56  ; v151 = 1
;;                                     v156 = iconst.i64 96
;;                                     v157 = iadd v24, v156  ; v156 = 96
;; @0062                               v62 = load.i64 notrap aligned region19 v157
;;                                     v158 = iconst.i64 -24
;;                                     v159 = iadd v62, v158  ; v158 = -24
;;                                     v160 = iconst.i64 0x0001_0000_0000
;; @0062                               v65 = stack_switch v159, v159, v160  ; v160 = 0x0001_0000_0000
;; @0062                               v67 = load.i64 notrap aligned region4 v25+88
;; @0062                               v68 = load.i64 notrap aligned region4 v25+96
;; @0062                               store notrap aligned region4 v26, v25+88
;; @0062                               store notrap aligned region4 v27, v25+96
;; @0062                               store notrap aligned region6 v151, v27+32  ; v151 = 1
;;                                     v161 = iconst.i32 0
;; @0062                               store notrap aligned region17 v161, v154  ; v161 = 0
;; @0062                               store notrap aligned region14 v161, v154+4  ; v161 = 0
;; @0062                               store notrap aligned region15 v149, v154+8  ; v149 = 0
;; @0062                               store notrap aligned region18 v149, v27+56  ; v149 = 0
;; @0062                               brif v65, block9, block6
;;
;;                                 block9:
;; @0062                               v58 = iconst.i64 32
;; @0062                               v75 = ushr.i64 v65, v58  ; v58 = 32
;; @0062                               v76 = iconst.i64 4
;; @0062                               v77 = icmp eq v75, v76  ; v76 = 4
;; @0062                               brif v77, block8, block7
;;
;;                                 block8 cold:
;; @0062                               v80 = iconst.i32 5
;; @0062                               store notrap aligned region6 v80, v68+32  ; v80 = 5
;; @0062                               v83 = load.i64 notrap aligned region13 v27
;; @0062                               store notrap aligned region1 v83, v25+24
;; @0062                               v84 = load.i64 notrap aligned region10 v27+8
;; @0062                               store notrap aligned region7 v84, v25+72
;; @0062                               v85 = load.i64 notrap aligned region11 v27+16
;; @0062                               store notrap aligned region8 v85, v25+64
;; @0062                               v86 = load.i64 notrap aligned region12 v27+24
;; @0062                               store notrap aligned region9 v86, v25+80
;;                                     v167 = iconst.i32 0
;;                                     v168 = iconst.i64 120
;;                                     v169 = iadd.i64 v68, v168  ; v168 = 120
;; @0062                               store notrap aligned region17 v167, v169  ; v167 = 0
;; @0062                               store notrap aligned region14 v167, v169+4  ; v167 = 0
;;                                     v170 = iconst.i64 0
;; @0062                               store notrap aligned region15 v170, v169+8  ; v170 = 0
;;                                     v171 = iconst.i64 136
;;                                     v172 = iadd.i64 v68, v171  ; v171 = 136
;; @0062                               store notrap aligned region17 v167, v172  ; v167 = 0
;; @0062                               store notrap aligned region14 v167, v172+4  ; v167 = 0
;; @0062                               store notrap aligned region15 v170, v172+8  ; v170 = 0
;; @0062                               try_call fn2(v0), sig2, block11, [ context v0 ]
;;
;;                                 block11:
;; @0062                               trap user12
;;
;;                                 block7:
;; @0062                               v101 = load.i64 notrap aligned region7 v25+72
;; @0062                               v102 = load.i64 notrap aligned region8 v25+64
;; @0062                               v103 = load.i64 notrap aligned region9 v25+80
;; @0062                               store notrap aligned region10 v101, v68+8
;; @0062                               store notrap aligned region11 v102, v68+16
;; @0062                               store notrap aligned region12 v103, v68+24
;; @0062                               v106 = load.i64 notrap aligned region13 v27
;; @0062                               store notrap aligned region1 v106, v25+24
;; @0062                               v107 = load.i64 notrap aligned region10 v27+8
;; @0062                               store notrap aligned region7 v107, v25+72
;; @0062                               v108 = load.i64 notrap aligned region11 v27+16
;; @0062                               store notrap aligned region8 v108, v25+64
;; @0062                               v109 = load.i64 notrap aligned region12 v27+24
;; @0062                               store notrap aligned region9 v109, v25+80
;; @0062                               v111 = load.i64 notrap aligned region2 v68+88
;; @0062                               jump block10
;;
;;                                 block12 cold:
;; @0062                               trap user12
;;
;;                                 block13:
;; @0062                               v118 = iconst.i64 136
;; @0062                               v119 = iadd.i64 v68, v118  ; v118 = 136
;; @0062                               v120 = load.i64 notrap aligned region15 v119+8
;; @0062                               v121 = load.i32 notrap aligned region16 v120
;;                                     v164 = iconst.i32 0
;; @0062                               store notrap aligned region17 v164, v119  ; v164 = 0
;; @0062                               jump block4
;;
;;                                 block10:
;; @0062                               v110 = ireduce.i32 v65
;; @0062                               br_table v110, block12, [block13]
;;
;;                                 block6:
;; @0062                               v125 = load.i64 notrap aligned region13 v27
;; @0062                               store notrap aligned region1 v125, v25+24
;; @0062                               v126 = load.i64 notrap aligned region10 v27+8
;; @0062                               store notrap aligned region7 v126, v25+72
;; @0062                               v127 = load.i64 notrap aligned region11 v27+16
;; @0062                               store notrap aligned region8 v127, v25+64
;; @0062                               v128 = load.i64 notrap aligned region12 v27+24
;; @0062                               store notrap aligned region9 v128, v25+80
;; @0062                               v131 = iconst.i32 4
;; @0062                               store notrap aligned region6 v131, v68+32  ; v131 = 4
;; @0062                               v132 = iconst.i64 120
;; @0062                               v133 = iadd.i64 v68, v132  ; v132 = 120
;; @0062                               v134 = load.i64 notrap aligned region15 v133+8
;;                                     v162 = iconst.i32 0
;; @0062                               store notrap aligned region17 v162, v133  ; v162 = 0
;; @0062                               store notrap aligned region14 v162, v133+4  ; v162 = 0
;;                                     v163 = iconst.i64 0
;; @0062                               store notrap aligned region15 v163, v133+8  ; v163 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v113 = uextend.i128 v111
;;                                     v165 = iconst.i64 64
;;                                     v166 = ishl v113, v165  ; v165 = 64
;; @0062                               v112 = uextend.i128 v68
;; @0062                               v117 = bor v166, v112
;; @006d                               jump block2(v117)
;; }
