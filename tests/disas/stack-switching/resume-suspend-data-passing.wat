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
;; @0044                               v31 = iconst.i64 144
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
;;                                     v94 = iconst.i64 144
;;                                     v95 = iadd.i64 v9, v94  ; v94 = 144
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
;;     region7 = 1140850688 "ContinuationStackMemory+0x0"
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
;;                                     v149 = ishl v10, v11  ; v11 = 64
;; @0058                               v9 = uextend.i128 v7
;; @0058                               v14 = bor v149, v9
;; @0062                               v23 = iconst.i64 1
;; @0062                               v26 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0062                               v29 = iconst.i64 0
;; @0062                               v30 = iconst.i64 2
;; @0062                               v34 = iconst.i32 1
;; @0062                               v35 = iconst.i64 32
;; @0062                               v37 = iconst.i32 2
;; @0062                               v53 = iconst.i64 40
;; @0062                               v56 = stack_addr.i64 ss0
;; @0062                               v57 = iconst.i64 48
;; @0062                               v58 = iadd v0, v57  ; v57 = 48
;; @0062                               v65 = iconst.i64 96
;; @0062                               v68 = iconst.i64 -24
;;                                     v153 = iconst.i64 0x0001_0000_0000
;; @005c                               jump block2(v14)
;;
;;                                 block2(v15: i128):
;; @0062                               jump block5
;;
;;                                 block5:
;; @0062                               v16 = ireduce.i64 v15
;; @0062                               trapz v16, user16
;; @0062                               v21 = load.i64 notrap aligned region2 v16+88
;;                                     v156 = iconst.i64 64
;;                                     v157 = ushr.i128 v15, v156  ; v156 = 64
;; @0062                               v20 = ireduce.i64 v157
;; @0062                               v22 = icmp eq v21, v20
;; @0062                               trapz v22, user23
;;                                     v158 = iconst.i64 1
;;                                     v159 = iadd v21, v158  ; v158 = 1
;; @0062                               store notrap aligned region2 v159, v16+88
;; @0062                               v25 = load.i64 notrap aligned region2 v16+80
;; @0062                               v27 = load.i64 notrap aligned region3 v26+88
;; @0062                               v28 = load.i64 notrap aligned region3 v26+96
;; @0062                               store notrap aligned region2 v27, v25+64
;; @0062                               store notrap aligned region2 v28, v25+72
;;                                     v160 = iconst.i64 0
;; @0062                               store notrap aligned region2 v160, v16+80  ; v160 = 0
;;                                     v161 = iconst.i64 2
;; @0062                               store notrap aligned region3 v161, v26+88  ; v161 = 2
;; @0062                               store notrap aligned region3 v16, v26+96
;;                                     v162 = iconst.i32 1
;;                                     v163 = iconst.i64 32
;;                                     v164 = iadd v16, v163  ; v163 = 32
;; @0062                               store notrap aligned region2 v162, v164  ; v162 = 1
;;                                     v165 = iconst.i32 2
;;                                     v166 = iadd v28, v163  ; v163 = 32
;; @0062                               store notrap aligned region2 v165, v166  ; v165 = 2
;; @0062                               v43 = load.i64 notrap aligned region4 v26+72
;; @0062                               v44 = load.i64 notrap aligned region5 v26+64
;; @0062                               v45 = load.i64 notrap aligned region6 v26+80
;; @0062                               store notrap aligned region2 v43, v28+8
;; @0062                               store notrap aligned region2 v44, v28+16
;; @0062                               store notrap aligned region2 v45, v28+24
;; @0062                               v46 = load.i64 notrap aligned region1 v26+24
;; @0062                               store notrap aligned region2 v46, v28
;; @0062                               v49 = load.i64 notrap aligned region2 v16
;; @0062                               store notrap aligned region1 v49, v26+24
;; @0062                               v50 = load.i64 notrap aligned region2 v16+8
;; @0062                               store notrap aligned region4 v50, v26+72
;; @0062                               v51 = load.i64 notrap aligned region2 v16+16
;; @0062                               store notrap aligned region5 v51, v26+64
;; @0062                               v52 = load.i64 notrap aligned region2 v16+24
;; @0062                               store notrap aligned region6 v52, v26+80
;;                                     v167 = iconst.i64 40
;;                                     v168 = iadd v28, v167  ; v167 = 40
;; @0062                               store notrap aligned region2 v162, v168+4  ; v162 = 1
;; @0062                               store.i64 notrap aligned region2 v56, v168+8
;;                                     v169 = iadd.i64 v0, v57  ; v57 = 48
;; @0062                               store notrap aligned region7 v169, v56
;; @0062                               store notrap aligned region2 v162, v168  ; v162 = 1
;; @0062                               store notrap aligned region2 v162, v28+56  ; v162 = 1
;;                                     v170 = iconst.i64 96
;;                                     v171 = iadd v25, v170  ; v170 = 96
;; @0062                               v67 = load.i64 notrap aligned region2 v171
;;                                     v172 = iconst.i64 -24
;;                                     v173 = iadd v67, v172  ; v172 = -24
;;                                     v174 = iconst.i64 0x0001_0000_0000
;; @0062                               v70 = stack_switch v173, v173, v174  ; v174 = 0x0001_0000_0000
;; @0062                               v72 = load.i64 notrap aligned region3 v26+88
;; @0062                               v73 = load.i64 notrap aligned region3 v26+96
;; @0062                               store notrap aligned region3 v27, v26+88
;; @0062                               store notrap aligned region3 v28, v26+96
;; @0062                               store notrap aligned region2 v162, v166  ; v162 = 1
;;                                     v175 = iconst.i32 0
;; @0062                               store notrap aligned region2 v175, v168  ; v175 = 0
;; @0062                               store notrap aligned region2 v175, v168+4  ; v175 = 0
;; @0062                               store notrap aligned region2 v160, v168+8  ; v160 = 0
;; @0062                               store notrap aligned region2 v160, v28+56  ; v160 = 0
;; @0062                               brif v70, block9, block6
;;
;;                                 block9:
;;                                     v180 = iconst.i64 32
;;                                     v181 = ushr.i64 v70, v180  ; v180 = 32
;; @0062                               v83 = iconst.i64 4
;; @0062                               v84 = icmp eq v181, v83  ; v83 = 4
;; @0062                               brif v84, block8, block7
;;
;;                                 block8 cold:
;; @0062                               v87 = iconst.i32 5
;;                                     v185 = iconst.i64 32
;;                                     v186 = iadd.i64 v73, v185  ; v185 = 32
;; @0062                               store notrap aligned region2 v87, v186  ; v87 = 5
;; @0062                               v92 = load.i64 notrap aligned region2 v28
;; @0062                               store notrap aligned region1 v92, v26+24
;; @0062                               v93 = load.i64 notrap aligned region2 v28+8
;; @0062                               store notrap aligned region4 v93, v26+72
;; @0062                               v94 = load.i64 notrap aligned region2 v28+16
;; @0062                               store notrap aligned region5 v94, v26+64
;; @0062                               v95 = load.i64 notrap aligned region2 v28+24
;; @0062                               store notrap aligned region6 v95, v26+80
;;                                     v187 = iconst.i32 0
;;                                     v188 = iconst.i64 120
;;                                     v189 = iadd.i64 v73, v188  ; v188 = 120
;; @0062                               store notrap aligned region2 v187, v189  ; v187 = 0
;; @0062                               store notrap aligned region2 v187, v189+4  ; v187 = 0
;;                                     v190 = iconst.i64 0
;; @0062                               store notrap aligned region2 v190, v189+8  ; v190 = 0
;;                                     v191 = iconst.i64 144
;;                                     v192 = iadd.i64 v73, v191  ; v191 = 144
;; @0062                               store notrap aligned region2 v187, v192  ; v187 = 0
;; @0062                               store notrap aligned region2 v187, v192+4  ; v187 = 0
;; @0062                               store notrap aligned region2 v190, v192+8  ; v190 = 0
;; @0062                               try_call fn2(v0), sig2, block11, [ context v0 ]
;;
;;                                 block11:
;; @0062                               trap user12
;;
;;                                 block7:
;; @0062                               v110 = load.i64 notrap aligned region4 v26+72
;; @0062                               v111 = load.i64 notrap aligned region5 v26+64
;; @0062                               v112 = load.i64 notrap aligned region6 v26+80
;; @0062                               store notrap aligned region2 v110, v73+8
;; @0062                               store notrap aligned region2 v111, v73+16
;; @0062                               store notrap aligned region2 v112, v73+24
;; @0062                               v115 = load.i64 notrap aligned region2 v28
;; @0062                               store notrap aligned region1 v115, v26+24
;; @0062                               v116 = load.i64 notrap aligned region2 v28+8
;; @0062                               store notrap aligned region4 v116, v26+72
;; @0062                               v117 = load.i64 notrap aligned region2 v28+16
;; @0062                               store notrap aligned region5 v117, v26+64
;; @0062                               v118 = load.i64 notrap aligned region2 v28+24
;; @0062                               store notrap aligned region6 v118, v26+80
;; @0062                               v120 = load.i64 notrap aligned region2 v73+88
;; @0062                               jump block10
;;
;;                                 block12 cold:
;; @0062                               trap user12
;;
;;                                 block13:
;; @0062                               v127 = iconst.i64 144
;; @0062                               v128 = iadd.i64 v73, v127  ; v127 = 144
;; @0062                               v129 = load.i64 notrap aligned region2 v128+8
;; @0062                               v130 = load.i32 notrap aligned region7 v129
;;                                     v182 = iconst.i32 0
;; @0062                               store notrap aligned region2 v182, v128  ; v182 = 0
;; @0062                               jump block4
;;
;;                                 block10:
;; @0062                               v119 = ireduce.i32 v70
;; @0062                               br_table v119, block12, [block13]
;;
;;                                 block6:
;; @0062                               v134 = load.i64 notrap aligned region2 v28
;; @0062                               store notrap aligned region1 v134, v26+24
;; @0062                               v135 = load.i64 notrap aligned region2 v28+8
;; @0062                               store notrap aligned region4 v135, v26+72
;; @0062                               v136 = load.i64 notrap aligned region2 v28+16
;; @0062                               store notrap aligned region5 v136, v26+64
;; @0062                               v137 = load.i64 notrap aligned region2 v28+24
;; @0062                               store notrap aligned region6 v137, v26+80
;; @0062                               v140 = iconst.i32 4
;;                                     v176 = iconst.i64 32
;;                                     v177 = iadd.i64 v73, v176  ; v176 = 32
;; @0062                               store notrap aligned region2 v140, v177  ; v140 = 4
;; @0062                               v143 = iconst.i64 120
;; @0062                               v144 = iadd.i64 v73, v143  ; v143 = 120
;; @0062                               v145 = load.i64 notrap aligned region2 v144+8
;;                                     v178 = iconst.i32 0
;; @0062                               store notrap aligned region2 v178, v144  ; v178 = 0
;; @0062                               store notrap aligned region2 v178, v144+4  ; v178 = 0
;;                                     v179 = iconst.i64 0
;; @0062                               store notrap aligned region2 v179, v144+8  ; v179 = 0
;; @0068                               return
;;
;;                                 block4:
;; @0062                               v122 = uextend.i128 v120
;;                                     v183 = iconst.i64 64
;;                                     v184 = ishl v122, v183  ; v183 = 64
;; @0062                               v121 = uextend.i128 v73
;; @0062                               v126 = bor v184, v121
;; @006d                               jump block2(v126)
;; }
