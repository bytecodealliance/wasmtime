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
;; @003b                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003b                               v5 = load.i64 notrap aligned region2 v4+88
;; @003b                               v6 = load.i64 notrap aligned region2 v4+96
;; @003b                               v9 = iconst.i64 1
;; @003b                               v13 = iconst.i64 40
;; @003b                               v17 = iconst.i32 0
;; @003b                               jump block2(v5, v6)
;;
;;                                 block2(v7: i64, v8: i64):
;;                                     v73 = iconst.i64 1
;;                                     v74 = icmp eq v7, v73  ; v73 = 1
;; @003b                               trapnz v74, user22
;; @003b                               jump block3
;;
;;                                 block3:
;; @003b                               v11 = load.i64 notrap aligned region3 v8+64
;; @003b                               v12 = load.i64 notrap aligned region3 v8+72
;;                                     v75 = iconst.i64 40
;;                                     v76 = iadd v12, v75  ; v75 = 40
;; @003b                               v15 = load.i64 notrap aligned region3 v76+8
;; @003b                               v16 = load.i32 notrap aligned region3 v12+56
;;                                     v77 = iconst.i32 0
;;                                     v67 = iconst.i32 3
;; @003b                               v2 = iconst.i64 48
;; @003b                               v3 = iadd.i64 v0, v2  ; v2 = 48
;; @003b                               v26 = iconst.i32 1
;; @003b                               jump block4(v77)  ; v77 = 0
;;
;;                                 block4(v18: i32):
;; @003b                               v19 = icmp ult v18, v16
;; @003b                               brif v19, block5, block2(v11, v12)
;;
;;                                 block5:
;;                                     v78 = iconst.i32 3
;;                                     v79 = ishl.i32 v18, v78  ; v78 = 3
;; @003b                               v22 = uextend.i64 v79
;; @003b                               v23 = iadd.i64 v15, v22
;; @003b                               v24 = load.i64 notrap aligned region4 v23
;;                                     v80 = iadd.i64 v0, v2  ; v2 = 48
;;                                     v81 = icmp eq v24, v80
;;                                     v82 = iconst.i32 1
;;                                     v83 = iadd.i32 v18, v82  ; v82 = 1
;; @003b                               brif v81, block6, block4(v83)
;;
;;                                 block6:
;; @003b                               store.i64 notrap aligned region3 v8, v6+80
;;                                     v84 = iconst.i32 1
;; @003b                               v28 = iconst.i64 136
;; @003b                               v29 = iadd.i64 v6, v28  ; v28 = 136
;; @003b                               store notrap aligned region3 v84, v29+4  ; v84 = 1
;; @003b                               v31 = stack_addr.i64 ss0
;; @003b                               store notrap aligned region3 v31, v29+8
;;                                     v85 = iconst.i32 3
;; @003b                               v35 = iconst.i64 32
;; @003b                               v36 = iadd.i64 v6, v35  ; v35 = 32
;; @003b                               store notrap aligned region3 v85, v36  ; v85 = 3
;; @003b                               v32 = iconst.i64 0
;; @003b                               store notrap aligned region3 v32, v8+64  ; v32 = 0
;; @003b                               store notrap aligned region3 v32, v8+72  ; v32 = 0
;; @003b                               v44 = iconst.i64 96
;; @003b                               v45 = iadd.i64 v8, v44  ; v44 = 96
;; @003b                               v46 = load.i64 notrap aligned region3 v45
;; @003b                               v47 = iconst.i64 -24
;; @003b                               v48 = iadd v46, v47  ; v47 = -24
;; @003b                               v42 = uextend.i64 v18
;;                                     v70 = iconst.i64 0x0002_0000_0000
;;                                     v71 = bor v42, v70  ; v70 = 0x0002_0000_0000
;; @003b                               v49 = stack_switch v48, v48, v71
;; @003b                               v51 = ushr v49, v35  ; v35 = 32
;; @003b                               v52 = iconst.i64 5
;; @003b                               v53 = icmp eq v51, v52  ; v52 = 5
;; @003b                               brif v53, block8, block9
;;
;;                                 block8 cold:
;;                                     v89 = iadd.i64 v6, v28  ; v28 = 136
;; @003b                               v56 = load.i64 notrap aligned region3 v89+8
;; @003b                               v57 = load.i32 notrap aligned region4 v56
;;                                     v90 = iconst.i32 0
;; @003b                               store notrap aligned region3 v90, v89  ; v90 = 0
;; @003b                               store notrap aligned region3 v90, v89+4  ; v90 = 0
;;                                     v91 = iconst.i64 0
;; @003b                               store notrap aligned region3 v91, v89+8  ; v91 = 0
;; @003b                               try_call fn0(v0, v57), sig0, block10, [ context v0 ]
;;
;;                                 block10:
;; @003b                               trap user12
;;
;;                                 block9:
;;                                     v86 = iadd.i64 v6, v28  ; v28 = 136
;; @003b                               v63 = load.i64 notrap aligned region3 v86+8
;;                                     v87 = iconst.i32 0
;; @003b                               store notrap aligned region3 v87, v86  ; v87 = 0
;; @003b                               store notrap aligned region3 v87, v86+4  ; v87 = 0
;;                                     v88 = iconst.i64 0
;; @003b                               store notrap aligned region3 v88, v86+8  ; v88 = 0
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
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
;; @0043                               v9 = iconst.i32 0
;; @0043                               v10 = call fn0(v0, v9)  ; v9 = 0
;; @0045                               trapz v10, user16
;; @0045                               v13 = call fn1(v0, v10, v9, v9)  ; v9 = 0, v9 = 0
;; @0045                               v14 = load.i64 notrap aligned region2 v13+88
;; @004e                               jump block3
;;
;;                                 block3:
;; @0045                               v16 = uextend.i128 v14
;; @0040                               v5 = iconst.i64 64
;;                                     v163 = ishl v16, v5  ; v5 = 64
;;                                     v165 = ireduce.i64 v163
;;                                     v167 = bor v165, v13
;; @004e                               trapz v167, user16
;; @004e                               v26 = load.i64 notrap aligned region2 v167+88
;; @0045                               v15 = uextend.i128 v13
;; @0045                               v20 = bor v163, v15
;;                                     v169 = ushr v20, v5  ; v5 = 64
;; @004e                               v25 = ireduce.i64 v169
;; @004e                               v27 = icmp eq v26, v25
;; @004e                               trapz v27, user23
;; @004e                               v28 = iconst.i64 1
;; @004e                               v29 = iadd v26, v28  ; v28 = 1
;; @004e                               store notrap aligned region2 v29, v167+88
;; @004e                               v30 = load.i64 notrap aligned region2 v167+80
;; @004e                               v31 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v32 = load.i64 notrap aligned region3 v31+88
;; @004e                               v33 = load.i64 notrap aligned region3 v31+96
;; @004e                               store notrap aligned region2 v32, v30+64
;; @004e                               store notrap aligned region2 v33, v30+72
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned region2 v2, v167+80  ; v2 = 0
;; @004e                               v35 = iconst.i64 2
;; @004e                               store notrap aligned region3 v35, v31+88  ; v35 = 2
;; @004e                               store notrap aligned region3 v167, v31+96
;; @004e                               v39 = iconst.i32 1
;; @004e                               v40 = iconst.i64 32
;; @004e                               v41 = iadd v167, v40  ; v40 = 32
;; @004e                               store notrap aligned region2 v39, v41  ; v39 = 1
;; @004e                               v42 = iconst.i32 2
;; @004e                               v44 = iadd v33, v40  ; v40 = 32
;; @004e                               store notrap aligned region2 v42, v44  ; v42 = 2
;; @004e                               v48 = load.i64 notrap aligned region4 v31+72
;; @004e                               v49 = load.i64 notrap aligned region5 v31+64
;; @004e                               v50 = load.i64 notrap aligned region6 v31+80
;; @004e                               store notrap aligned region7 v48, v33+8
;; @004e                               store notrap aligned region8 v49, v33+16
;; @004e                               store notrap aligned region9 v50, v33+24
;; @004e                               v51 = load.i64 notrap aligned region1 v31+24
;; @004e                               store notrap aligned region10 v51, v33
;; @004e                               v54 = load.i64 notrap aligned region10 v167
;; @004e                               store notrap aligned region1 v54, v31+24
;; @004e                               v55 = load.i64 notrap aligned region7 v167+8
;; @004e                               store notrap aligned region4 v55, v31+72
;; @004e                               v56 = load.i64 notrap aligned region8 v167+16
;; @004e                               store notrap aligned region5 v56, v31+64
;; @004e                               v57 = load.i64 notrap aligned region9 v167+24
;; @004e                               store notrap aligned region6 v57, v31+80
;; @004e                               v58 = iconst.i64 40
;; @004e                               v59 = iadd v33, v58  ; v58 = 40
;; @004e                               store notrap aligned region2 v39, v59+4  ; v39 = 1
;; @004e                               v61 = stack_addr.i64 ss0
;; @004e                               store notrap aligned region2 v61, v59+8
;; @004e                               v62 = iconst.i64 48
;; @004e                               v63 = iadd.i64 v0, v62  ; v62 = 48
;; @004e                               store notrap aligned region11 v63, v61
;; @004e                               store notrap aligned region2 v39, v59  ; v39 = 1
;; @004e                               store notrap aligned region2 v39, v33+56  ; v39 = 1
;; @004e                               v70 = iconst.i64 96
;; @004e                               v71 = iadd v30, v70  ; v70 = 96
;; @004e                               v72 = load.i64 notrap aligned region2 v71
;; @004e                               v73 = iconst.i64 -24
;; @004e                               v74 = iadd v72, v73  ; v73 = -24
;;                                     v171 = iconst.i64 0x0001_0000_0000
;; @004e                               v75 = stack_switch v74, v74, v171  ; v171 = 0x0001_0000_0000
;; @004e                               v77 = load.i64 notrap aligned region3 v31+88
;; @004e                               v78 = load.i64 notrap aligned region3 v31+96
;; @004e                               store notrap aligned region3 v32, v31+88
;; @004e                               store notrap aligned region3 v33, v31+96
;; @004e                               store notrap aligned region2 v39, v44  ; v39 = 1
;;                                     v174 = iconst.i32 0
;; @004e                               store notrap aligned region2 v174, v59  ; v174 = 0
;; @004e                               store notrap aligned region2 v174, v59+4  ; v174 = 0
;; @004e                               store notrap aligned region2 v2, v59+8  ; v2 = 0
;; @004e                               store notrap aligned region2 v2, v33+56  ; v2 = 0
;; @004e                               brif v75, block7, block4
;;
;;                                 block7:
;;                                     v182 = iconst.i64 32
;;                                     v183 = ushr.i64 v75, v182  ; v182 = 32
;; @004e                               v88 = iconst.i64 4
;; @004e                               v89 = icmp eq v183, v88  ; v88 = 4
;; @004e                               brif v89, block6, block5
;;
;;                                 block6 cold:
;; @004e                               v92 = iconst.i32 5
;;                                     v187 = iconst.i64 32
;;                                     v188 = iadd.i64 v78, v187  ; v187 = 32
;; @004e                               store notrap aligned region2 v92, v188  ; v92 = 5
;; @004e                               v97 = load.i64 notrap aligned region10 v33
;; @004e                               store notrap aligned region1 v97, v31+24
;; @004e                               v98 = load.i64 notrap aligned region7 v33+8
;; @004e                               store notrap aligned region4 v98, v31+72
;; @004e                               v99 = load.i64 notrap aligned region8 v33+16
;; @004e                               store notrap aligned region5 v99, v31+64
;; @004e                               v100 = load.i64 notrap aligned region9 v33+24
;; @004e                               store notrap aligned region6 v100, v31+80
;;                                     v189 = iconst.i32 0
;;                                     v190 = iconst.i64 120
;;                                     v191 = iadd.i64 v78, v190  ; v190 = 120
;; @004e                               store notrap aligned region2 v189, v191  ; v189 = 0
;; @004e                               store notrap aligned region2 v189, v191+4  ; v189 = 0
;;                                     v192 = iconst.i64 0
;; @004e                               store notrap aligned region2 v192, v191+8  ; v192 = 0
;;                                     v193 = iconst.i64 136
;;                                     v194 = iadd.i64 v78, v193  ; v193 = 136
;; @004e                               store notrap aligned region2 v189, v194  ; v189 = 0
;; @004e                               store notrap aligned region2 v189, v194+4  ; v189 = 0
;; @004e                               store notrap aligned region2 v192, v194+8  ; v192 = 0
;; @004e                               try_call fn2(v0), sig2, block9, [ context v0 ]
;;
;;                                 block9:
;; @004e                               trap user12
;;
;;                                 block5:
;; @004e                               v115 = load.i64 notrap aligned region4 v31+72
;; @004e                               v116 = load.i64 notrap aligned region5 v31+64
;; @004e                               v117 = load.i64 notrap aligned region6 v31+80
;; @004e                               store notrap aligned region7 v115, v78+8
;; @004e                               store notrap aligned region8 v116, v78+16
;; @004e                               store notrap aligned region9 v117, v78+24
;; @004e                               v120 = load.i64 notrap aligned region10 v33
;; @004e                               store notrap aligned region1 v120, v31+24
;; @004e                               v121 = load.i64 notrap aligned region7 v33+8
;; @004e                               store notrap aligned region4 v121, v31+72
;; @004e                               v122 = load.i64 notrap aligned region8 v33+16
;; @004e                               store notrap aligned region5 v122, v31+64
;; @004e                               v123 = load.i64 notrap aligned region9 v33+24
;; @004e                               store notrap aligned region6 v123, v31+80
;; @004e                               v125 = load.i64 notrap aligned region2 v78+88
;; @004e                               jump block8
;;
;;                                 block10 cold:
;; @004e                               trap user12
;;
;;                                 block11:
;; @004e                               v132 = iconst.i64 136
;; @004e                               v133 = iadd.i64 v78, v132  ; v132 = 136
;; @004e                               v134 = load.i64 notrap aligned region2 v133+8
;;                                     v184 = iconst.i32 0
;; @004e                               store notrap aligned region2 v184, v133  ; v184 = 0
;; @004e                               v127 = uextend.i128 v125
;;                                     v185 = iconst.i64 64
;;                                     v186 = ishl v127, v185  ; v185 = 64
;; @004e                               v126 = uextend.i128 v78
;; @004e                               v131 = bor v186, v126
;; @004e                               jump block2(v131)
;;
;;                                 block8:
;; @004e                               v124 = ireduce.i32 v75
;; @004e                               br_table v124, block10, [block11]
;;
;;                                 block4:
;; @004e                               v138 = load.i64 notrap aligned region10 v33
;; @004e                               store notrap aligned region1 v138, v31+24
;; @004e                               v139 = load.i64 notrap aligned region7 v33+8
;; @004e                               store notrap aligned region4 v139, v31+72
;; @004e                               v140 = load.i64 notrap aligned region8 v33+16
;; @004e                               store notrap aligned region5 v140, v31+64
;; @004e                               v141 = load.i64 notrap aligned region9 v33+24
;; @004e                               store notrap aligned region6 v141, v31+80
;; @004e                               v144 = iconst.i32 4
;;                                     v175 = iconst.i64 32
;;                                     v176 = iadd.i64 v78, v175  ; v175 = 32
;; @004e                               store notrap aligned region2 v144, v176  ; v144 = 4
;; @004e                               v147 = iconst.i64 120
;; @004e                               v148 = iadd.i64 v78, v147  ; v147 = 120
;; @004e                               v149 = load.i64 notrap aligned region2 v148+8
;;                                     v177 = iconst.i32 0
;; @004e                               store notrap aligned region2 v177, v148  ; v177 = 0
;; @004e                               store notrap aligned region2 v177, v148+4  ; v177 = 0
;;                                     v178 = iconst.i64 0
;; @004e                               store notrap aligned region2 v178, v148+8  ; v178 = 0
;;                                     v179 = uextend.i128 v178  ; v178 = 0
;;                                     v180 = iconst.i64 64
;;                                     v181 = ishl v179, v180  ; v180 = 64
;; @0040                               v8 = bor v181, v179
;; @0056                               jump block2(v8)
;;
;;                                 block2(v160: i128):
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
