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
;; @003b                               v28 = iconst.i64 144
;; @003b                               v29 = iadd.i64 v6, v28  ; v28 = 144
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
;;                                     v89 = iadd.i64 v6, v28  ; v28 = 144
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
;;                                     v86 = iadd.i64 v6, v28  ; v28 = 144
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
;; @0043                               v9 = iconst.i32 0
;; @0043                               v10 = call fn0(v0, v9)  ; v9 = 0
;; @0045                               trapz v10, user16
;; @0045                               v14 = call fn1(v0, v10, v9, v9, v9)  ; v9 = 0, v9 = 0, v9 = 0
;; @0045                               v15 = load.i64 notrap aligned region2 v14+88
;; @004e                               jump block3
;;
;;                                 block3:
;; @0045                               v17 = uextend.i128 v15
;; @0040                               v5 = iconst.i64 64
;;                                     v164 = ishl v17, v5  ; v5 = 64
;;                                     v166 = ireduce.i64 v164
;;                                     v168 = bor v166, v14
;; @004e                               trapz v168, user16
;; @004e                               v27 = load.i64 notrap aligned region2 v168+88
;; @0045                               v16 = uextend.i128 v14
;; @0045                               v21 = bor v164, v16
;;                                     v170 = ushr v21, v5  ; v5 = 64
;; @004e                               v26 = ireduce.i64 v170
;; @004e                               v28 = icmp eq v27, v26
;; @004e                               trapz v28, user23
;; @004e                               v29 = iconst.i64 1
;; @004e                               v30 = iadd v27, v29  ; v29 = 1
;; @004e                               store notrap aligned region2 v30, v168+88
;; @004e                               v31 = load.i64 notrap aligned region2 v168+80
;; @004e                               v32 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v33 = load.i64 notrap aligned region3 v32+88
;; @004e                               v34 = load.i64 notrap aligned region3 v32+96
;; @004e                               store notrap aligned region2 v33, v31+64
;; @004e                               store notrap aligned region2 v34, v31+72
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned region2 v2, v168+80  ; v2 = 0
;; @004e                               v36 = iconst.i64 2
;; @004e                               store notrap aligned region3 v36, v32+88  ; v36 = 2
;; @004e                               store notrap aligned region3 v168, v32+96
;; @004e                               v40 = iconst.i32 1
;; @004e                               v41 = iconst.i64 32
;; @004e                               v42 = iadd v168, v41  ; v41 = 32
;; @004e                               store notrap aligned region2 v40, v42  ; v40 = 1
;; @004e                               v43 = iconst.i32 2
;; @004e                               v45 = iadd v34, v41  ; v41 = 32
;; @004e                               store notrap aligned region2 v43, v45  ; v43 = 2
;; @004e                               v49 = load.i64 notrap aligned region4 v32+72
;; @004e                               v50 = load.i64 notrap aligned region5 v32+64
;; @004e                               v51 = load.i64 notrap aligned region6 v32+80
;; @004e                               store notrap aligned region2 v49, v34+8
;; @004e                               store notrap aligned region2 v50, v34+16
;; @004e                               store notrap aligned region2 v51, v34+24
;; @004e                               v52 = load.i64 notrap aligned region1 v32+24
;; @004e                               store notrap aligned region2 v52, v34
;; @004e                               v55 = load.i64 notrap aligned region2 v168
;; @004e                               store notrap aligned region1 v55, v32+24
;; @004e                               v56 = load.i64 notrap aligned region2 v168+8
;; @004e                               store notrap aligned region4 v56, v32+72
;; @004e                               v57 = load.i64 notrap aligned region2 v168+16
;; @004e                               store notrap aligned region5 v57, v32+64
;; @004e                               v58 = load.i64 notrap aligned region2 v168+24
;; @004e                               store notrap aligned region6 v58, v32+80
;; @004e                               v59 = iconst.i64 40
;; @004e                               v60 = iadd v34, v59  ; v59 = 40
;; @004e                               store notrap aligned region2 v40, v60+4  ; v40 = 1
;; @004e                               v62 = stack_addr.i64 ss0
;; @004e                               store notrap aligned region2 v62, v60+8
;; @004e                               v63 = iconst.i64 48
;; @004e                               v64 = iadd.i64 v0, v63  ; v63 = 48
;; @004e                               store notrap aligned region7 v64, v62
;; @004e                               store notrap aligned region2 v40, v60  ; v40 = 1
;; @004e                               store notrap aligned region2 v40, v34+56  ; v40 = 1
;; @004e                               v71 = iconst.i64 96
;; @004e                               v72 = iadd v31, v71  ; v71 = 96
;; @004e                               v73 = load.i64 notrap aligned region2 v72
;; @004e                               v74 = iconst.i64 -24
;; @004e                               v75 = iadd v73, v74  ; v74 = -24
;;                                     v172 = iconst.i64 0x0001_0000_0000
;; @004e                               v76 = stack_switch v75, v75, v172  ; v172 = 0x0001_0000_0000
;; @004e                               v78 = load.i64 notrap aligned region3 v32+88
;; @004e                               v79 = load.i64 notrap aligned region3 v32+96
;; @004e                               store notrap aligned region3 v33, v32+88
;; @004e                               store notrap aligned region3 v34, v32+96
;; @004e                               store notrap aligned region2 v40, v45  ; v40 = 1
;;                                     v175 = iconst.i32 0
;; @004e                               store notrap aligned region2 v175, v60  ; v175 = 0
;; @004e                               store notrap aligned region2 v175, v60+4  ; v175 = 0
;; @004e                               store notrap aligned region2 v2, v60+8  ; v2 = 0
;; @004e                               store notrap aligned region2 v2, v34+56  ; v2 = 0
;; @004e                               brif v76, block7, block4
;;
;;                                 block7:
;;                                     v183 = iconst.i64 32
;;                                     v184 = ushr.i64 v76, v183  ; v183 = 32
;; @004e                               v89 = iconst.i64 4
;; @004e                               v90 = icmp eq v184, v89  ; v89 = 4
;; @004e                               brif v90, block6, block5
;;
;;                                 block6 cold:
;; @004e                               v93 = iconst.i32 5
;;                                     v188 = iconst.i64 32
;;                                     v189 = iadd.i64 v79, v188  ; v188 = 32
;; @004e                               store notrap aligned region2 v93, v189  ; v93 = 5
;; @004e                               v98 = load.i64 notrap aligned region2 v34
;; @004e                               store notrap aligned region1 v98, v32+24
;; @004e                               v99 = load.i64 notrap aligned region2 v34+8
;; @004e                               store notrap aligned region4 v99, v32+72
;; @004e                               v100 = load.i64 notrap aligned region2 v34+16
;; @004e                               store notrap aligned region5 v100, v32+64
;; @004e                               v101 = load.i64 notrap aligned region2 v34+24
;; @004e                               store notrap aligned region6 v101, v32+80
;;                                     v190 = iconst.i32 0
;;                                     v191 = iconst.i64 120
;;                                     v192 = iadd.i64 v79, v191  ; v191 = 120
;; @004e                               store notrap aligned region2 v190, v192  ; v190 = 0
;; @004e                               store notrap aligned region2 v190, v192+4  ; v190 = 0
;;                                     v193 = iconst.i64 0
;; @004e                               store notrap aligned region2 v193, v192+8  ; v193 = 0
;;                                     v194 = iconst.i64 144
;;                                     v195 = iadd.i64 v79, v194  ; v194 = 144
;; @004e                               store notrap aligned region2 v190, v195  ; v190 = 0
;; @004e                               store notrap aligned region2 v190, v195+4  ; v190 = 0
;; @004e                               store notrap aligned region2 v193, v195+8  ; v193 = 0
;; @004e                               try_call fn2(v0), sig2, block9, [ context v0 ]
;;
;;                                 block9:
;; @004e                               trap user12
;;
;;                                 block5:
;; @004e                               v116 = load.i64 notrap aligned region4 v32+72
;; @004e                               v117 = load.i64 notrap aligned region5 v32+64
;; @004e                               v118 = load.i64 notrap aligned region6 v32+80
;; @004e                               store notrap aligned region2 v116, v79+8
;; @004e                               store notrap aligned region2 v117, v79+16
;; @004e                               store notrap aligned region2 v118, v79+24
;; @004e                               v121 = load.i64 notrap aligned region2 v34
;; @004e                               store notrap aligned region1 v121, v32+24
;; @004e                               v122 = load.i64 notrap aligned region2 v34+8
;; @004e                               store notrap aligned region4 v122, v32+72
;; @004e                               v123 = load.i64 notrap aligned region2 v34+16
;; @004e                               store notrap aligned region5 v123, v32+64
;; @004e                               v124 = load.i64 notrap aligned region2 v34+24
;; @004e                               store notrap aligned region6 v124, v32+80
;; @004e                               v126 = load.i64 notrap aligned region2 v79+88
;; @004e                               jump block8
;;
;;                                 block10 cold:
;; @004e                               trap user12
;;
;;                                 block11:
;; @004e                               v133 = iconst.i64 144
;; @004e                               v134 = iadd.i64 v79, v133  ; v133 = 144
;; @004e                               v135 = load.i64 notrap aligned region2 v134+8
;;                                     v185 = iconst.i32 0
;; @004e                               store notrap aligned region2 v185, v134  ; v185 = 0
;; @004e                               v128 = uextend.i128 v126
;;                                     v186 = iconst.i64 64
;;                                     v187 = ishl v128, v186  ; v186 = 64
;; @004e                               v127 = uextend.i128 v79
;; @004e                               v132 = bor v187, v127
;; @004e                               jump block2(v132)
;;
;;                                 block8:
;; @004e                               v125 = ireduce.i32 v76
;; @004e                               br_table v125, block10, [block11]
;;
;;                                 block4:
;; @004e                               v139 = load.i64 notrap aligned region2 v34
;; @004e                               store notrap aligned region1 v139, v32+24
;; @004e                               v140 = load.i64 notrap aligned region2 v34+8
;; @004e                               store notrap aligned region4 v140, v32+72
;; @004e                               v141 = load.i64 notrap aligned region2 v34+16
;; @004e                               store notrap aligned region5 v141, v32+64
;; @004e                               v142 = load.i64 notrap aligned region2 v34+24
;; @004e                               store notrap aligned region6 v142, v32+80
;; @004e                               v145 = iconst.i32 4
;;                                     v176 = iconst.i64 32
;;                                     v177 = iadd.i64 v79, v176  ; v176 = 32
;; @004e                               store notrap aligned region2 v145, v177  ; v145 = 4
;; @004e                               v148 = iconst.i64 120
;; @004e                               v149 = iadd.i64 v79, v148  ; v148 = 120
;; @004e                               v150 = load.i64 notrap aligned region2 v149+8
;;                                     v178 = iconst.i32 0
;; @004e                               store notrap aligned region2 v178, v149  ; v178 = 0
;; @004e                               store notrap aligned region2 v178, v149+4  ; v178 = 0
;;                                     v179 = iconst.i64 0
;; @004e                               store notrap aligned region2 v179, v149+8  ; v179 = 0
;;                                     v180 = uextend.i128 v179  ; v179 = 0
;;                                     v181 = iconst.i64 64
;;                                     v182 = ishl v180, v181  ; v181 = 64
;; @0040                               v8 = bor v182, v180
;; @0056                               jump block2(v8)
;;
;;                                 block2(v161: i128):
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
