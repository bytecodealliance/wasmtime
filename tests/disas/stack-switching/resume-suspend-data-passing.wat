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
;;     sig0 = (i64 vmctx, i32) -> i8 tail
;;     fn0 = colocated u805306368:44 sig0
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
;;                                     v79 = iconst.i64 0x0002_0000_0000
;; @0044                               v57 = iconst.i64 5
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
;;                                 block4(v10: i64, v11: i64, v73: i32):
;;                                     v82 = iconst.i64 1
;;                                     v83 = icmp eq v10, v82  ; v82 = 1
;; @0044                               trapnz v83, user22
;; @0044                               jump block5
;;
;;                                 block5:
;; @0044                               v14 = load.i64 notrap aligned region3 v11+64
;; @0044                               v15 = load.i64 notrap aligned region3 v11+72
;;                                     v84 = iconst.i64 40
;;                                     v85 = iadd v15, v84  ; v84 = 40
;; @0044                               v18 = load.i64 notrap aligned region3 v85+8
;; @0044                               v19 = load.i32 notrap aligned region3 v15+56
;;                                     v86 = iconst.i32 0
;;                                     v76 = iconst.i32 3
;; @0044                               v5 = iconst.i64 48
;; @0044                               v6 = iadd.i64 v0, v5  ; v5 = 48
;; @0044                               v29 = iconst.i32 1
;; @0044                               jump block6(v86)  ; v86 = 0
;;
;;                                 block6(v21: i32):
;; @0044                               v22 = icmp ult v21, v19
;; @0044                               brif v22, block7, block4(v14, v15, v73)
;;
;;                                 block7:
;;                                     v87 = iconst.i32 3
;;                                     v88 = ishl.i32 v21, v87  ; v87 = 3
;; @0044                               v25 = uextend.i64 v88
;; @0044                               v26 = iadd.i64 v18, v25
;; @0044                               v27 = load.i64 notrap aligned region4 v26
;;                                     v89 = iadd.i64 v0, v5  ; v5 = 48
;;                                     v90 = icmp eq v27, v89
;;                                     v91 = iconst.i32 1
;;                                     v92 = iadd.i32 v21, v91  ; v91 = 1
;; @0044                               brif v90, block8, block6(v92)
;;
;;                                 block8:
;; @0044                               store.i64 notrap aligned region3 v11, v9+80
;;                                     v93 = iconst.i32 1
;;                                     v94 = iconst.i64 136
;;                                     v95 = iadd.i64 v9, v94  ; v94 = 136
;; @0044                               store notrap aligned region3 v93, v95+4  ; v93 = 1
;; @0044                               store.i64 notrap aligned region3 v34, v95+8
;; @0044                               store.i32 notrap aligned region4 v4, v34
;; @0044                               store notrap aligned region3 v93, v95  ; v93 = 1
;;                                     v96 = iconst.i32 3
;;                                     v97 = iconst.i64 32
;;                                     v98 = iadd.i64 v9, v97  ; v97 = 32
;; @0044                               store notrap aligned region3 v96, v98  ; v96 = 3
;;                                     v99 = iconst.i64 0
;; @0044                               store notrap aligned region3 v99, v11+64  ; v99 = 0
;; @0044                               store notrap aligned region3 v99, v11+72  ; v99 = 0
;;                                     v100 = iconst.i64 96
;;                                     v101 = iadd.i64 v11, v100  ; v100 = 96
;; @0044                               v51 = load.i64 notrap aligned region3 v101
;;                                     v102 = iconst.i64 -24
;;                                     v103 = iadd v51, v102  ; v102 = -24
;; @0044                               v47 = uextend.i64 v21
;;                                     v104 = iconst.i64 0x0002_0000_0000
;;                                     v105 = bor v47, v104  ; v104 = 0x0002_0000_0000
;; @0044                               v54 = stack_switch v103, v103, v105
;;                                     v106 = ushr v54, v97  ; v97 = 32
;;                                     v107 = iconst.i64 5
;;                                     v108 = icmp eq v106, v107  ; v107 = 5
;; @0044                               brif v108, block10, block11
;;
;;                                 block10 cold:
;; @0044                               v61 = load.i64 notrap aligned region3 v95+8
;; @0044                               v62 = load.i32 notrap aligned region4 v61
;;                                     v113 = iconst.i32 0
;; @0044                               store notrap aligned region3 v113, v95  ; v113 = 0
;; @0044                               store notrap aligned region3 v113, v95+4  ; v113 = 0
;;                                     v114 = iconst.i64 0
;; @0044                               store notrap aligned region3 v114, v95+8  ; v114 = 0
;; @0044                               try_call fn0(v0, v62), sig0, block12, [ context v0 ]
;;
;;                                 block12:
;; @0044                               trap user12
;;
;;                                 block11:
;; @0044                               v68 = load.i64 notrap aligned region3 v95+8
;;                                     v109 = iconst.i32 0
;; @0044                               store notrap aligned region3 v109, v95  ; v109 = 0
;; @0044                               store notrap aligned region3 v109, v95+4  ; v109 = 0
;;                                     v110 = iconst.i64 0
;; @0044                               store notrap aligned region3 v110, v95+8  ; v110 = 0
;;                                     v111 = iconst.i32 1
;;                                     v112 = isub.i32 v73, v111  ; v111 = 1
;; @004d                               brif v112, block2(v112), block13
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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1073741824 "VMContRef+0x0"
;;     region3 = 67108952 "VMStoreContext+0x58"
;;     region4 = 67108936 "VMStoreContext+0x48"
;;     region5 = 67108928 "VMStoreContext+0x40"
;;     region6 = 67108944 "VMStoreContext+0x50"
;;     region7 = 2415919112 "VMStackLimits+0x8"
;;     region8 = 2415919120 "VMStackLimits+0x10"
;;     region9 = 2415919128 "VMStackLimits+0x18"
;;     region10 = 2415919104 "VMStackLimits+0x0"
;;     region11 = 1140850688 "ContinuationStackMemory+0x0"
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
;;                                     v148 = ishl v9, v10  ; v10 = 64
;; @0058                               v8 = uextend.i128 v6
;; @0058                               v13 = bor v148, v8
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
;;                                     v152 = iconst.i64 0x0001_0000_0000
;; @005c                               jump block2(v13)
;;
;;                                 block2(v14: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v15 = ireduce.i64 v14
;; @0062                               trapz v15, user16
;; @0062                               v20 = load.i64 notrap aligned region2 v15+88
;;                                     v155 = iconst.i64 64
;;                                     v156 = ushr.i128 v14, v155  ; v155 = 64
;; @0062                               v19 = ireduce.i64 v156
;; @0062                               v21 = icmp eq v20, v19
;; @0062                               trapz v21, user23
;;                                     v157 = iconst.i64 1
;;                                     v158 = iadd v20, v157  ; v157 = 1
;; @0062                               store notrap aligned region2 v158, v15+88
;; @0062                               v24 = load.i64 notrap aligned region2 v15+80
;; @0062                               v26 = load.i64 notrap aligned region3 v25+88
;; @0062                               v27 = load.i64 notrap aligned region3 v25+96
;; @0062                               store notrap aligned region2 v26, v24+64
;; @0062                               store notrap aligned region2 v27, v24+72
;;                                     v159 = iconst.i64 0
;; @0062                               store notrap aligned region2 v159, v15+80  ; v159 = 0
;;                                     v160 = iconst.i64 2
;; @0062                               store notrap aligned region3 v160, v25+88  ; v160 = 2
;; @0062                               store notrap aligned region3 v15, v25+96
;;                                     v161 = iconst.i32 1
;;                                     v162 = iconst.i64 32
;;                                     v163 = iadd v15, v162  ; v162 = 32
;; @0062                               store notrap aligned region2 v161, v163  ; v161 = 1
;;                                     v164 = iconst.i32 2
;;                                     v165 = iadd v27, v162  ; v162 = 32
;; @0062                               store notrap aligned region2 v164, v165  ; v164 = 2
;; @0062                               v42 = load.i64 notrap aligned region4 v25+72
;; @0062                               v43 = load.i64 notrap aligned region5 v25+64
;; @0062                               v44 = load.i64 notrap aligned region6 v25+80
;; @0062                               store notrap aligned region7 v42, v27+8
;; @0062                               store notrap aligned region8 v43, v27+16
;; @0062                               store notrap aligned region9 v44, v27+24
;; @0062                               v45 = load.i64 notrap aligned region1 v25+24
;; @0062                               store notrap aligned region10 v45, v27
;; @0062                               v48 = load.i64 notrap aligned region10 v15
;; @0062                               store notrap aligned region1 v48, v25+24
;; @0062                               v49 = load.i64 notrap aligned region7 v15+8
;; @0062                               store notrap aligned region4 v49, v25+72
;; @0062                               v50 = load.i64 notrap aligned region8 v15+16
;; @0062                               store notrap aligned region5 v50, v25+64
;; @0062                               v51 = load.i64 notrap aligned region9 v15+24
;; @0062                               store notrap aligned region6 v51, v25+80
;;                                     v166 = iconst.i64 40
;;                                     v167 = iadd v27, v166  ; v166 = 40
;; @0062                               store notrap aligned region2 v161, v167+4  ; v161 = 1
;; @0062                               store.i64 notrap aligned region2 v55, v167+8
;;                                     v168 = iadd.i64 v0, v56  ; v56 = 48
;; @0062                               store notrap aligned region11 v168, v55
;; @0062                               store notrap aligned region2 v161, v167  ; v161 = 1
;; @0062                               store notrap aligned region2 v161, v27+56  ; v161 = 1
;;                                     v169 = iconst.i64 96
;;                                     v170 = iadd v24, v169  ; v169 = 96
;; @0062                               v66 = load.i64 notrap aligned region2 v170
;;                                     v171 = iconst.i64 -24
;;                                     v172 = iadd v66, v171  ; v171 = -24
;;                                     v173 = iconst.i64 0x0001_0000_0000
;; @0062                               v69 = stack_switch v172, v172, v173  ; v173 = 0x0001_0000_0000
;; @0062                               v71 = load.i64 notrap aligned region3 v25+88
;; @0062                               v72 = load.i64 notrap aligned region3 v25+96
;; @0062                               store notrap aligned region3 v26, v25+88
;; @0062                               store notrap aligned region3 v27, v25+96
;; @0062                               store notrap aligned region2 v161, v165  ; v161 = 1
;;                                     v174 = iconst.i32 0
;; @0062                               store notrap aligned region2 v174, v167  ; v174 = 0
;; @0062                               store notrap aligned region2 v174, v167+4  ; v174 = 0
;; @0062                               store notrap aligned region2 v159, v167+8  ; v159 = 0
;; @0062                               store notrap aligned region2 v159, v27+56  ; v159 = 0
;; @0062                               brif v69, block9, block6
;;
;;                                 block9:
;;                                     v179 = iconst.i64 32
;;                                     v180 = ushr.i64 v69, v179  ; v179 = 32
;; @0062                               v82 = iconst.i64 4
;; @0062                               v83 = icmp eq v180, v82  ; v82 = 4
;; @0062                               brif v83, block8, block7
;;
;;                                 block8 cold:
;; @0062                               v86 = iconst.i32 5
;;                                     v184 = iconst.i64 32
;;                                     v185 = iadd.i64 v72, v184  ; v184 = 32
;; @0062                               store notrap aligned region2 v86, v185  ; v86 = 5
;; @0062                               v91 = load.i64 notrap aligned region10 v27
;; @0062                               store notrap aligned region1 v91, v25+24
;; @0062                               v92 = load.i64 notrap aligned region7 v27+8
;; @0062                               store notrap aligned region4 v92, v25+72
;; @0062                               v93 = load.i64 notrap aligned region8 v27+16
;; @0062                               store notrap aligned region5 v93, v25+64
;; @0062                               v94 = load.i64 notrap aligned region9 v27+24
;; @0062                               store notrap aligned region6 v94, v25+80
;;                                     v186 = iconst.i32 0
;;                                     v187 = iconst.i64 120
;;                                     v188 = iadd.i64 v72, v187  ; v187 = 120
;; @0062                               store notrap aligned region2 v186, v188  ; v186 = 0
;; @0062                               store notrap aligned region2 v186, v188+4  ; v186 = 0
;;                                     v189 = iconst.i64 0
;; @0062                               store notrap aligned region2 v189, v188+8  ; v189 = 0
;;                                     v190 = iconst.i64 136
;;                                     v191 = iadd.i64 v72, v190  ; v190 = 136
;; @0062                               store notrap aligned region2 v186, v191  ; v186 = 0
;; @0062                               store notrap aligned region2 v186, v191+4  ; v186 = 0
;; @0062                               store notrap aligned region2 v189, v191+8  ; v189 = 0
;; @0062                               try_call fn2(v0), sig2, block11, [ context v0 ]
;;
;;                                 block11:
;; @0062                               trap user12
;;
;;                                 block7:
;; @0062                               v109 = load.i64 notrap aligned region4 v25+72
;; @0062                               v110 = load.i64 notrap aligned region5 v25+64
;; @0062                               v111 = load.i64 notrap aligned region6 v25+80
;; @0062                               store notrap aligned region7 v109, v72+8
;; @0062                               store notrap aligned region8 v110, v72+16
;; @0062                               store notrap aligned region9 v111, v72+24
;; @0062                               v114 = load.i64 notrap aligned region10 v27
;; @0062                               store notrap aligned region1 v114, v25+24
;; @0062                               v115 = load.i64 notrap aligned region7 v27+8
;; @0062                               store notrap aligned region4 v115, v25+72
;; @0062                               v116 = load.i64 notrap aligned region8 v27+16
;; @0062                               store notrap aligned region5 v116, v25+64
;; @0062                               v117 = load.i64 notrap aligned region9 v27+24
;; @0062                               store notrap aligned region6 v117, v25+80
;; @0062                               v119 = load.i64 notrap aligned region2 v72+88
;; @0062                               jump block10
;;
;;                                 block12 cold:
;; @0062                               trap user12
;;
;;                                 block13:
;; @0062                               v126 = iconst.i64 136
;; @0062                               v127 = iadd.i64 v72, v126  ; v126 = 136
;; @0062                               v128 = load.i64 notrap aligned region2 v127+8
;; @0062                               v129 = load.i32 notrap aligned region11 v128
;;                                     v181 = iconst.i32 0
;; @0062                               store notrap aligned region2 v181, v127  ; v181 = 0
;; @0062                               jump block4
;;
;;                                 block10:
;; @0062                               v118 = ireduce.i32 v69
;; @0062                               br_table v118, block12, [block13]
;;
;;                                 block6:
;; @0062                               v133 = load.i64 notrap aligned region10 v27
;; @0062                               store notrap aligned region1 v133, v25+24
;; @0062                               v134 = load.i64 notrap aligned region7 v27+8
;; @0062                               store notrap aligned region4 v134, v25+72
;; @0062                               v135 = load.i64 notrap aligned region8 v27+16
;; @0062                               store notrap aligned region5 v135, v25+64
;; @0062                               v136 = load.i64 notrap aligned region9 v27+24
;; @0062                               store notrap aligned region6 v136, v25+80
;; @0062                               v139 = iconst.i32 4
;;                                     v175 = iconst.i64 32
;;                                     v176 = iadd.i64 v72, v175  ; v175 = 32
;; @0062                               store notrap aligned region2 v139, v176  ; v139 = 4
;; @0062                               v142 = iconst.i64 120
;; @0062                               v143 = iadd.i64 v72, v142  ; v142 = 120
;; @0062                               v144 = load.i64 notrap aligned region2 v143+8
;;                                     v177 = iconst.i32 0
;; @0062                               store notrap aligned region2 v177, v143  ; v177 = 0
;; @0062                               store notrap aligned region2 v177, v143+4  ; v177 = 0
;;                                     v178 = iconst.i64 0
;; @0062                               store notrap aligned region2 v178, v143+8  ; v178 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v121 = uextend.i128 v119
;;                                     v182 = iconst.i64 64
;;                                     v183 = ishl v121, v182  ; v182 = 64
;; @0062                               v120 = uextend.i128 v72
;; @0062                               v125 = bor v183, v120
;; @006d                               jump block2(v125)
;; }
