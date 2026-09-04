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
;;     region3 = 1073741888 "VMContRef+0x40"
;;     region4 = 2550136840 "VMHostArray+0x8"
;;     region5 = 2483028024 "VMCommonStackInformation+0x38"
;;     region6 = 1140850688 "ContinuationStackMemory+0x0"
;;     region7 = 1073741904 "VMContRef+0x50"
;;     region8 = 2550136836 "VMHostArray+0x4"
;;     region9 = 2483028000 "VMCommonStackInformation+0x20"
;;     region10 = 1073741920 "VMContRef+0x60"
;;     region11 = 2550136832 "VMHostArray+0x0"
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
;;                                     v71 = iconst.i64 1
;;                                     v72 = icmp eq v7, v71  ; v71 = 1
;; @003b                               trapnz v72, user22
;; @003b                               jump block3
;;
;;                                 block3:
;; @003b                               v11 = load.i64 notrap aligned region3 v8+64
;; @003b                               v12 = load.i64 notrap aligned region3 v8+72
;;                                     v73 = iconst.i64 40
;;                                     v74 = iadd v12, v73  ; v73 = 40
;; @003b                               v15 = load.i64 notrap aligned region4 v74+8
;; @003b                               v16 = load.i32 notrap aligned region5 v12+56
;;                                     v75 = iconst.i32 0
;;                                     v65 = iconst.i32 3
;; @003b                               v2 = iconst.i64 48
;; @003b                               v3 = iadd.i64 v0, v2  ; v2 = 48
;; @003b                               v26 = iconst.i32 1
;; @003b                               jump block4(v75)  ; v75 = 0
;;
;;                                 block4(v18: i32):
;; @003b                               v19 = icmp ult v18, v16
;; @003b                               brif v19, block5, block2(v11, v12)
;;
;;                                 block5:
;;                                     v76 = iconst.i32 3
;;                                     v77 = ishl.i32 v18, v76  ; v76 = 3
;; @003b                               v22 = uextend.i64 v77
;; @003b                               v23 = iadd.i64 v15, v22
;; @003b                               v24 = load.i64 notrap aligned region6 v23
;;                                     v78 = iadd.i64 v0, v2  ; v2 = 48
;;                                     v79 = icmp eq v24, v78
;;                                     v80 = iconst.i32 1
;;                                     v81 = iadd.i32 v18, v80  ; v80 = 1
;; @003b                               brif v79, block6, block4(v81)
;;
;;                                 block6:
;; @003b                               store.i64 notrap aligned region7 v8, v6+80
;;                                     v82 = iconst.i32 1
;; @003b                               v28 = iconst.i64 136
;; @003b                               v29 = iadd.i64 v6, v28  ; v28 = 136
;; @003b                               store notrap aligned region8 v82, v29+4  ; v82 = 1
;; @003b                               v31 = stack_addr.i64 ss0
;; @003b                               store notrap aligned region4 v31, v29+8
;;                                     v83 = iconst.i32 3
;; @003b                               store notrap aligned region9 v83, v6+32  ; v83 = 3
;; @003b                               v32 = iconst.i64 0
;; @003b                               store notrap aligned region3 v32, v8+64  ; v32 = 0
;; @003b                               store notrap aligned region3 v32, v8+72  ; v32 = 0
;; @003b                               v42 = iconst.i64 96
;; @003b                               v43 = iadd.i64 v8, v42  ; v42 = 96
;; @003b                               v44 = load.i64 notrap aligned region10 v43
;; @003b                               v45 = iconst.i64 -24
;; @003b                               v46 = iadd v44, v45  ; v45 = -24
;; @003b                               v40 = uextend.i64 v18
;;                                     v68 = iconst.i64 0x0002_0000_0000
;;                                     v69 = bor v40, v68  ; v68 = 0x0002_0000_0000
;; @003b                               v47 = stack_switch v46, v46, v69
;; @003b                               v38 = iconst.i64 32
;; @003b                               v49 = ushr v47, v38  ; v38 = 32
;; @003b                               v50 = iconst.i64 5
;; @003b                               v51 = icmp eq v49, v50  ; v50 = 5
;; @003b                               brif v51, block8, block9
;;
;;                                 block8 cold:
;;                                     v87 = iadd.i64 v6, v28  ; v28 = 136
;; @003b                               v54 = load.i64 notrap aligned region4 v87+8
;; @003b                               v55 = load.i32 notrap aligned region6 v54
;;                                     v88 = iconst.i32 0
;; @003b                               store notrap aligned region11 v88, v87  ; v88 = 0
;; @003b                               store notrap aligned region8 v88, v87+4  ; v88 = 0
;;                                     v89 = iconst.i64 0
;; @003b                               store notrap aligned region4 v89, v87+8  ; v89 = 0
;; @003b                               try_call fn0(v0, v55), sig0, block10, [ context v0 ]
;;
;;                                 block10:
;; @003b                               trap user12
;;
;;                                 block9:
;;                                     v84 = iadd.i64 v6, v28  ; v28 = 136
;; @003b                               v61 = load.i64 notrap aligned region4 v84+8
;;                                     v85 = iconst.i32 0
;; @003b                               store notrap aligned region11 v85, v84  ; v85 = 0
;; @003b                               store notrap aligned region8 v85, v84+4  ; v85 = 0
;;                                     v86 = iconst.i64 0
;; @003b                               store notrap aligned region4 v86, v84+8  ; v86 = 0
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
;;     region2 = 1073741912 "VMContRef+0x58"
;;     region3 = 1073741904 "VMContRef+0x50"
;;     region4 = 67108952 "VMStoreContext+0x58"
;;     region5 = 1073741888 "VMContRef+0x40"
;;     region6 = 2483028000 "VMCommonStackInformation+0x20"
;;     region7 = 67108936 "VMStoreContext+0x48"
;;     region8 = 67108928 "VMStoreContext+0x40"
;;     region9 = 67108944 "VMStoreContext+0x50"
;;     region10 = 2415919112 "VMStackLimits+0x8"
;;     region11 = 2415919120 "VMStackLimits+0x10"
;;     region12 = 2415919128 "VMStackLimits+0x18"
;;     region13 = 2415919104 "VMStackLimits+0x0"
;;     region14 = 2550136836 "VMHostArray+0x4"
;;     region15 = 2550136840 "VMHostArray+0x8"
;;     region16 = 1140850688 "ContinuationStackMemory+0x0"
;;     region17 = 2550136832 "VMHostArray+0x0"
;;     region18 = 2483028024 "VMCommonStackInformation+0x38"
;;     region19 = 1073741920 "VMContRef+0x60"
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
;;                                     v153 = ishl v16, v5  ; v5 = 64
;;                                     v155 = ireduce.i64 v153
;;                                     v157 = bor v155, v13
;; @004e                               trapz v157, user16
;; @004e                               v26 = load.i64 notrap aligned region2 v157+88
;; @0045                               v15 = uextend.i128 v13
;; @0045                               v20 = bor v153, v15
;;                                     v159 = ushr v20, v5  ; v5 = 64
;; @004e                               v25 = ireduce.i64 v159
;; @004e                               v27 = icmp eq v26, v25
;; @004e                               trapz v27, user23
;; @004e                               v28 = iconst.i64 1
;; @004e                               v29 = iadd v26, v28  ; v28 = 1
;; @004e                               store notrap aligned region2 v29, v157+88
;; @004e                               v30 = load.i64 notrap aligned region3 v157+80
;; @004e                               v31 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v32 = load.i64 notrap aligned region4 v31+88
;; @004e                               v33 = load.i64 notrap aligned region4 v31+96
;; @004e                               store notrap aligned region5 v32, v30+64
;; @004e                               store notrap aligned region5 v33, v30+72
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned region3 v2, v157+80  ; v2 = 0
;; @004e                               v35 = iconst.i64 2
;; @004e                               store notrap aligned region4 v35, v31+88  ; v35 = 2
;; @004e                               store notrap aligned region4 v157, v31+96
;; @004e                               v39 = iconst.i32 1
;; @004e                               store notrap aligned region6 v39, v157+32  ; v39 = 1
;; @004e                               v40 = iconst.i32 2
;; @004e                               store notrap aligned region6 v40, v33+32  ; v40 = 2
;; @004e                               v44 = load.i64 notrap aligned region7 v31+72
;; @004e                               v45 = load.i64 notrap aligned region8 v31+64
;; @004e                               v46 = load.i64 notrap aligned region9 v31+80
;; @004e                               store notrap aligned region10 v44, v33+8
;; @004e                               store notrap aligned region11 v45, v33+16
;; @004e                               store notrap aligned region12 v46, v33+24
;; @004e                               v47 = load.i64 notrap aligned region1 v31+24
;; @004e                               store notrap aligned region13 v47, v33
;; @004e                               v50 = load.i64 notrap aligned region13 v157
;; @004e                               store notrap aligned region1 v50, v31+24
;; @004e                               v51 = load.i64 notrap aligned region10 v157+8
;; @004e                               store notrap aligned region7 v51, v31+72
;; @004e                               v52 = load.i64 notrap aligned region11 v157+16
;; @004e                               store notrap aligned region8 v52, v31+64
;; @004e                               v53 = load.i64 notrap aligned region12 v157+24
;; @004e                               store notrap aligned region9 v53, v31+80
;; @004e                               v54 = iconst.i64 40
;; @004e                               v55 = iadd v33, v54  ; v54 = 40
;; @004e                               store notrap aligned region14 v39, v55+4  ; v39 = 1
;; @004e                               v57 = stack_addr.i64 ss0
;; @004e                               store notrap aligned region15 v57, v55+8
;; @004e                               v58 = iconst.i64 48
;; @004e                               v59 = iadd.i64 v0, v58  ; v58 = 48
;; @004e                               store notrap aligned region16 v59, v57
;; @004e                               store notrap aligned region17 v39, v55  ; v39 = 1
;; @004e                               store notrap aligned region18 v39, v33+56  ; v39 = 1
;; @004e                               v66 = iconst.i64 96
;; @004e                               v67 = iadd v30, v66  ; v66 = 96
;; @004e                               v68 = load.i64 notrap aligned region19 v67
;; @004e                               v69 = iconst.i64 -24
;; @004e                               v70 = iadd v68, v69  ; v69 = -24
;;                                     v161 = iconst.i64 0x0001_0000_0000
;; @004e                               v71 = stack_switch v70, v70, v161  ; v161 = 0x0001_0000_0000
;; @004e                               v73 = load.i64 notrap aligned region4 v31+88
;; @004e                               v74 = load.i64 notrap aligned region4 v31+96
;; @004e                               store notrap aligned region4 v32, v31+88
;; @004e                               store notrap aligned region4 v33, v31+96
;; @004e                               store notrap aligned region6 v39, v33+32  ; v39 = 1
;;                                     v164 = iconst.i32 0
;; @004e                               store notrap aligned region17 v164, v55  ; v164 = 0
;; @004e                               store notrap aligned region14 v164, v55+4  ; v164 = 0
;; @004e                               store notrap aligned region15 v2, v55+8  ; v2 = 0
;; @004e                               store notrap aligned region18 v2, v33+56  ; v2 = 0
;; @004e                               brif v71, block7, block4
;;
;;                                 block7:
;; @004e                               v64 = iconst.i64 32
;; @004e                               v81 = ushr.i64 v71, v64  ; v64 = 32
;; @004e                               v82 = iconst.i64 4
;; @004e                               v83 = icmp eq v81, v82  ; v82 = 4
;; @004e                               brif v83, block6, block5
;;
;;                                 block6 cold:
;; @004e                               v86 = iconst.i32 5
;; @004e                               store notrap aligned region6 v86, v74+32  ; v86 = 5
;; @004e                               v89 = load.i64 notrap aligned region13 v33
;; @004e                               store notrap aligned region1 v89, v31+24
;; @004e                               v90 = load.i64 notrap aligned region10 v33+8
;; @004e                               store notrap aligned region7 v90, v31+72
;; @004e                               v91 = load.i64 notrap aligned region11 v33+16
;; @004e                               store notrap aligned region8 v91, v31+64
;; @004e                               v92 = load.i64 notrap aligned region12 v33+24
;; @004e                               store notrap aligned region9 v92, v31+80
;;                                     v173 = iconst.i32 0
;;                                     v174 = iconst.i64 120
;;                                     v175 = iadd.i64 v74, v174  ; v174 = 120
;; @004e                               store notrap aligned region17 v173, v175  ; v173 = 0
;; @004e                               store notrap aligned region14 v173, v175+4  ; v173 = 0
;;                                     v176 = iconst.i64 0
;; @004e                               store notrap aligned region15 v176, v175+8  ; v176 = 0
;;                                     v177 = iconst.i64 136
;;                                     v178 = iadd.i64 v74, v177  ; v177 = 136
;; @004e                               store notrap aligned region17 v173, v178  ; v173 = 0
;; @004e                               store notrap aligned region14 v173, v178+4  ; v173 = 0
;; @004e                               store notrap aligned region15 v176, v178+8  ; v176 = 0
;; @004e                               try_call fn2(v0), sig2, block9, [ context v0 ]
;;
;;                                 block9:
;; @004e                               trap user12
;;
;;                                 block5:
;; @004e                               v107 = load.i64 notrap aligned region7 v31+72
;; @004e                               v108 = load.i64 notrap aligned region8 v31+64
;; @004e                               v109 = load.i64 notrap aligned region9 v31+80
;; @004e                               store notrap aligned region10 v107, v74+8
;; @004e                               store notrap aligned region11 v108, v74+16
;; @004e                               store notrap aligned region12 v109, v74+24
;; @004e                               v112 = load.i64 notrap aligned region13 v33
;; @004e                               store notrap aligned region1 v112, v31+24
;; @004e                               v113 = load.i64 notrap aligned region10 v33+8
;; @004e                               store notrap aligned region7 v113, v31+72
;; @004e                               v114 = load.i64 notrap aligned region11 v33+16
;; @004e                               store notrap aligned region8 v114, v31+64
;; @004e                               v115 = load.i64 notrap aligned region12 v33+24
;; @004e                               store notrap aligned region9 v115, v31+80
;; @004e                               v117 = load.i64 notrap aligned region2 v74+88
;; @004e                               jump block8
;;
;;                                 block10 cold:
;; @004e                               trap user12
;;
;;                                 block11:
;; @004e                               v124 = iconst.i64 136
;; @004e                               v125 = iadd.i64 v74, v124  ; v124 = 136
;; @004e                               v126 = load.i64 notrap aligned region15 v125+8
;;                                     v170 = iconst.i32 0
;; @004e                               store notrap aligned region17 v170, v125  ; v170 = 0
;; @004e                               v119 = uextend.i128 v117
;;                                     v171 = iconst.i64 64
;;                                     v172 = ishl v119, v171  ; v171 = 64
;; @004e                               v118 = uextend.i128 v74
;; @004e                               v123 = bor v172, v118
;; @004e                               jump block2(v123)
;;
;;                                 block8:
;; @004e                               v116 = ireduce.i32 v71
;; @004e                               br_table v116, block10, [block11]
;;
;;                                 block4:
;; @004e                               v130 = load.i64 notrap aligned region13 v33
;; @004e                               store notrap aligned region1 v130, v31+24
;; @004e                               v131 = load.i64 notrap aligned region10 v33+8
;; @004e                               store notrap aligned region7 v131, v31+72
;; @004e                               v132 = load.i64 notrap aligned region11 v33+16
;; @004e                               store notrap aligned region8 v132, v31+64
;; @004e                               v133 = load.i64 notrap aligned region12 v33+24
;; @004e                               store notrap aligned region9 v133, v31+80
;; @004e                               v136 = iconst.i32 4
;; @004e                               store notrap aligned region6 v136, v74+32  ; v136 = 4
;; @004e                               v137 = iconst.i64 120
;; @004e                               v138 = iadd.i64 v74, v137  ; v137 = 120
;; @004e                               v139 = load.i64 notrap aligned region15 v138+8
;;                                     v165 = iconst.i32 0
;; @004e                               store notrap aligned region17 v165, v138  ; v165 = 0
;; @004e                               store notrap aligned region14 v165, v138+4  ; v165 = 0
;;                                     v166 = iconst.i64 0
;; @004e                               store notrap aligned region15 v166, v138+8  ; v166 = 0
;;                                     v167 = uextend.i128 v166  ; v166 = 0
;;                                     v168 = iconst.i64 64
;;                                     v169 = ishl v167, v168  ; v168 = 64
;; @0040                               v8 = bor v169, v167
;; @0056                               jump block2(v8)
;;
;;                                 block2(v150: i128):
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
