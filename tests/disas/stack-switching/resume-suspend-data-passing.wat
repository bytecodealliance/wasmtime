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
;; @0044                               v31 = iconst.i64 136
;; @0044                               v34 = stack_addr.i64 ss0
;; @0044                               v40 = iconst.i64 32
;; @0044                               v37 = iconst.i64 0
;; @0044                               v49 = iconst.i64 96
;; @0044                               v52 = iconst.i64 -24
;;                                     v68 = iconst.i64 0x0002_0000_0000
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
;;                                 block4(v10: i64, v11: i64, v62: i32):
;;                                     v71 = iconst.i64 1
;;                                     v72 = icmp eq v10, v71  ; v71 = 1
;; @0044                               trapnz v72, user22
;; @0044                               jump block5
;;
;;                                 block5:
;; @0044                               v14 = load.i64 notrap aligned region3 v11+64
;; @0044                               v15 = load.i64 notrap aligned region3 v11+72
;;                                     v73 = iconst.i64 40
;;                                     v74 = iadd v15, v73  ; v73 = 40
;; @0044                               v18 = load.i64 notrap aligned region3 v74+8
;; @0044                               v19 = load.i32 notrap aligned region3 v15+56
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
;; @0044                               store.i64 notrap aligned region3 v11, v9+80
;;                                     v82 = iconst.i32 1
;;                                     v83 = iconst.i64 136
;;                                     v84 = iadd.i64 v9, v83  ; v83 = 136
;; @0044                               store notrap aligned region3 v82, v84+4  ; v82 = 1
;; @0044                               store.i64 notrap aligned region3 v34, v84+8
;; @0044                               store.i32 notrap aligned region4 v4, v34
;; @0044                               store notrap aligned region3 v82, v84  ; v82 = 1
;;                                     v85 = iconst.i32 3
;;                                     v86 = iconst.i64 32
;;                                     v87 = iadd.i64 v9, v86  ; v86 = 32
;; @0044                               store notrap aligned region3 v85, v87  ; v85 = 3
;;                                     v88 = iconst.i64 0
;; @0044                               store notrap aligned region3 v88, v11+64  ; v88 = 0
;; @0044                               store notrap aligned region3 v88, v11+72  ; v88 = 0
;;                                     v89 = iconst.i64 96
;;                                     v90 = iadd.i64 v11, v89  ; v89 = 96
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
;;     region5 = 67108928 "VMStoreContext+0x40"
;;     region6 = 67108944 "VMStoreContext+0x50"
;;     region7 = 1140850688 "ContinuationStackMemory+0x0"
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
;;                                     v150 = ishl v9, v10  ; v10 = 64
;; @0058                               v8 = uextend.i128 v6
;; @0058                               v13 = bor v150, v8
;; @0062                               v22 = iconst.i64 1
;; @0062                               v25 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0062                               v28 = iconst.i64 0
;; @0062                               v29 = iconst.i64 2
;; @0062                               v33 = iconst.i32 1
;; @0062                               v34 = iconst.i64 32
;; @0062                               v36 = iconst.i32 2
;; @0062                               v52 = iconst.i64 40
;; @0062                               v55 = stack_addr.i64 ss0
;; @0062                               v56 = iconst.i64 48
;; @0062                               v57 = iadd v0, v56  ; v56 = 48
;; @0062                               v64 = iconst.i64 96
;; @0062                               v67 = iconst.i64 -24
;;                                     v154 = iconst.i64 0x0001_0000_0000
;; @0062                               v82 = iconst.i64 4
;; @005c                               jump block2(v13)
;;
;;                                 block2(v14: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v15 = ireduce.i64 v14
;; @0062                               trapz v15, user16
;; @0062                               v20 = load.i64 notrap aligned region2 v15+88
;;                                     v157 = iconst.i64 64
;;                                     v158 = ushr.i128 v14, v157  ; v157 = 64
;; @0062                               v19 = ireduce.i64 v158
;; @0062                               v21 = icmp eq v20, v19
;; @0062                               trapz v21, user23
;;                                     v159 = iconst.i64 1
;;                                     v160 = iadd v20, v159  ; v159 = 1
;; @0062                               store notrap aligned region2 v160, v15+88
;; @0062                               v24 = load.i64 notrap aligned region2 v15+80
;; @0062                               v26 = load.i64 notrap aligned region3 v25+88
;; @0062                               v27 = load.i64 notrap aligned region3 v25+96
;; @0062                               store notrap aligned region2 v26, v24+64
;; @0062                               store notrap aligned region2 v27, v24+72
;;                                     v161 = iconst.i64 0
;; @0062                               store notrap aligned region2 v161, v15+80  ; v161 = 0
;;                                     v162 = iconst.i64 2
;; @0062                               store notrap aligned region3 v162, v25+88  ; v162 = 2
;; @0062                               store notrap aligned region3 v15, v25+96
;;                                     v163 = iconst.i32 1
;;                                     v164 = iconst.i64 32
;;                                     v165 = iadd v15, v164  ; v164 = 32
;; @0062                               store notrap aligned region2 v163, v165  ; v163 = 1
;;                                     v166 = iconst.i32 2
;;                                     v167 = iadd v27, v164  ; v164 = 32
;; @0062                               store notrap aligned region2 v166, v167  ; v166 = 2
;; @0062                               v42 = load.i64 notrap aligned region4 v25+72
;; @0062                               v43 = load.i64 notrap aligned region5 v25+64
;; @0062                               v44 = load.i64 notrap aligned region6 v25+80
;; @0062                               store notrap aligned region2 v42, v27+8
;; @0062                               store notrap aligned region2 v43, v27+16
;; @0062                               store notrap aligned region2 v44, v27+24
;; @0062                               v45 = load.i64 notrap aligned region1 v25+24
;; @0062                               store notrap aligned region2 v45, v27
;; @0062                               v48 = load.i64 notrap aligned region2 v15
;; @0062                               store notrap aligned region1 v48, v25+24
;; @0062                               v49 = load.i64 notrap aligned region2 v15+8
;; @0062                               store notrap aligned region4 v49, v25+72
;; @0062                               v50 = load.i64 notrap aligned region2 v15+16
;; @0062                               store notrap aligned region5 v50, v25+64
;; @0062                               v51 = load.i64 notrap aligned region2 v15+24
;; @0062                               store notrap aligned region6 v51, v25+80
;;                                     v168 = iconst.i64 40
;;                                     v169 = iadd v27, v168  ; v168 = 40
;; @0062                               store notrap aligned region2 v163, v169+4  ; v163 = 1
;; @0062                               store.i64 notrap aligned region2 v55, v169+8
;;                                     v170 = iadd.i64 v0, v56  ; v56 = 48
;; @0062                               store notrap aligned region7 v170, v55
;; @0062                               store notrap aligned region2 v163, v169  ; v163 = 1
;; @0062                               store notrap aligned region2 v163, v27+56  ; v163 = 1
;;                                     v171 = iconst.i64 96
;;                                     v172 = iadd v24, v171  ; v171 = 96
;; @0062                               v66 = load.i64 notrap aligned region2 v172
;;                                     v173 = iconst.i64 -24
;;                                     v174 = iadd v66, v173  ; v173 = -24
;;                                     v175 = iconst.i64 0x0001_0000_0000
;; @0062                               v69 = stack_switch v174, v174, v175  ; v175 = 0x0001_0000_0000
;; @0062                               v71 = load.i64 notrap aligned region3 v25+88
;; @0062                               v72 = load.i64 notrap aligned region3 v25+96
;; @0062                               store notrap aligned region3 v26, v25+88
;; @0062                               store notrap aligned region3 v27, v25+96
;; @0062                               store notrap aligned region2 v163, v167  ; v163 = 1
;;                                     v176 = iconst.i32 0
;; @0062                               store notrap aligned region2 v176, v169  ; v176 = 0
;; @0062                               store notrap aligned region2 v176, v169+4  ; v176 = 0
;; @0062                               store notrap aligned region2 v161, v169+8  ; v161 = 0
;; @0062                               store notrap aligned region2 v161, v27+56  ; v161 = 0
;;                                     v177 = ushr v69, v164  ; v164 = 32
;;                                     v178 = iconst.i64 4
;;                                     v179 = icmp eq v177, v178  ; v178 = 4
;; @0062                               brif v179, block8, block9
;;
;;                                 block9:
;; @0062                               brif.i64 v177, block7, block6
;;
;;                                 block8 cold:
;; @0062                               v88 = iconst.i32 5
;;                                     v187 = iconst.i64 32
;;                                     v188 = iadd.i64 v72, v187  ; v187 = 32
;; @0062                               store notrap aligned region2 v88, v188  ; v88 = 5
;; @0062                               v93 = load.i64 notrap aligned region2 v27
;; @0062                               store notrap aligned region1 v93, v25+24
;; @0062                               v94 = load.i64 notrap aligned region2 v27+8
;; @0062                               store notrap aligned region4 v94, v25+72
;; @0062                               v95 = load.i64 notrap aligned region2 v27+16
;; @0062                               store notrap aligned region5 v95, v25+64
;; @0062                               v96 = load.i64 notrap aligned region2 v27+24
;; @0062                               store notrap aligned region6 v96, v25+80
;;                                     v189 = iconst.i32 0
;;                                     v190 = iconst.i64 120
;;                                     v191 = iadd.i64 v72, v190  ; v190 = 120
;; @0062                               store notrap aligned region2 v189, v191  ; v189 = 0
;; @0062                               store notrap aligned region2 v189, v191+4  ; v189 = 0
;;                                     v192 = iconst.i64 0
;; @0062                               store notrap aligned region2 v192, v191+8  ; v192 = 0
;;                                     v193 = iconst.i64 136
;;                                     v194 = iadd.i64 v72, v193  ; v193 = 136
;; @0062                               store notrap aligned region2 v189, v194  ; v189 = 0
;; @0062                               store notrap aligned region2 v189, v194+4  ; v189 = 0
;; @0062                               store notrap aligned region2 v192, v194+8  ; v192 = 0
;; @0062                               call fn2(v0)
;; @0062                               trap user1
;;
;;                                 block7:
;; @0062                               v111 = load.i64 notrap aligned region4 v25+72
;; @0062                               v112 = load.i64 notrap aligned region5 v25+64
;; @0062                               v113 = load.i64 notrap aligned region6 v25+80
;; @0062                               store notrap aligned region2 v111, v72+8
;; @0062                               store notrap aligned region2 v112, v72+16
;; @0062                               store notrap aligned region2 v113, v72+24
;; @0062                               v116 = load.i64 notrap aligned region2 v27
;; @0062                               store notrap aligned region1 v116, v25+24
;; @0062                               v117 = load.i64 notrap aligned region2 v27+8
;; @0062                               store notrap aligned region4 v117, v25+72
;; @0062                               v118 = load.i64 notrap aligned region2 v27+16
;; @0062                               store notrap aligned region5 v118, v25+64
;; @0062                               v119 = load.i64 notrap aligned region2 v27+24
;; @0062                               store notrap aligned region6 v119, v25+80
;; @0062                               v121 = load.i64 notrap aligned region2 v72+88
;; @0062                               jump block10
;;
;;                                 block11 cold:
;; @0062                               trap user12
;;
;;                                 block12:
;; @0062                               v128 = iconst.i64 136
;; @0062                               v129 = iadd.i64 v72, v128  ; v128 = 136
;; @0062                               v130 = load.i64 notrap aligned region2 v129+8
;; @0062                               v131 = load.i32 notrap aligned region7 v130
;;                                     v184 = iconst.i32 0
;; @0062                               store notrap aligned region2 v184, v129  ; v184 = 0
;; @0062                               jump block4
;;
;;                                 block10:
;; @0062                               v120 = ireduce.i32 v69
;; @0062                               br_table v120, block11, [block12]
;;
;;                                 block6:
;; @0062                               v135 = load.i64 notrap aligned region2 v27
;; @0062                               store notrap aligned region1 v135, v25+24
;; @0062                               v136 = load.i64 notrap aligned region2 v27+8
;; @0062                               store notrap aligned region4 v136, v25+72
;; @0062                               v137 = load.i64 notrap aligned region2 v27+16
;; @0062                               store notrap aligned region5 v137, v25+64
;; @0062                               v138 = load.i64 notrap aligned region2 v27+24
;; @0062                               store notrap aligned region6 v138, v25+80
;; @0062                               v141 = iconst.i32 4
;;                                     v180 = iconst.i64 32
;;                                     v181 = iadd.i64 v72, v180  ; v180 = 32
;; @0062                               store notrap aligned region2 v141, v181  ; v141 = 4
;; @0062                               v144 = iconst.i64 120
;; @0062                               v145 = iadd.i64 v72, v144  ; v144 = 120
;; @0062                               v146 = load.i64 notrap aligned region2 v145+8
;;                                     v182 = iconst.i32 0
;; @0062                               store notrap aligned region2 v182, v145  ; v182 = 0
;; @0062                               store notrap aligned region2 v182, v145+4  ; v182 = 0
;;                                     v183 = iconst.i64 0
;; @0062                               store notrap aligned region2 v183, v145+8  ; v183 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v123 = uextend.i128 v121
;;                                     v185 = iconst.i64 64
;;                                     v186 = ishl v123, v185  ; v185 = 64
;; @0062                               v122 = uextend.i128 v72
;; @0062                               v127 = bor v186, v122
;; @006d                               jump block2(v127)
;; }
