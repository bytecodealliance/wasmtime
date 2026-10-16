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
;;     region9 = 255 ""
;;     region10 = 221 ""
;;     region11 = 82 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) -> i8 tail
;;     fn0 = colocated u805306368:43 sig0
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
;; @003b                               v11 = load.i64 notrap aligned region3 v8+88
;; @003b                               v12 = load.i64 notrap aligned region3 v8+96
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
;; @003b                               store.i64 notrap aligned region7 v8, v6+112
;;                                     v82 = iconst.i32 1
;; @003b                               v28 = iconst.i64 168
;; @003b                               v29 = iadd.i64 v6, v28  ; v28 = 168
;; @003b                               store notrap aligned region8 v82, v29+4  ; v82 = 1
;; @003b                               v31 = stack_addr.i64 ss0
;; @003b                               store notrap aligned region4 v31, v29+8
;;                                     v83 = iconst.i32 3
;; @003b                               store notrap aligned region9 v83, v6+32  ; v83 = 3
;; @003b                               v32 = iconst.i64 0
;; @003b                               store notrap aligned region3 v32, v8+88  ; v32 = 0
;; @003b                               store notrap aligned region3 v32, v8+96  ; v32 = 0
;; @003b                               v42 = iconst.i64 120
;; @003b                               v43 = iadd.i64 v8, v42  ; v42 = 120
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
;;                                     v87 = iadd.i64 v6, v28  ; v28 = 168
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
;;                                     v84 = iadd.i64 v6, v28  ; v28 = 168
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
;; @0043                               v10 = func_addr.i64 fn1
;; @0043                               store notrap aligned region4 v10, v0+84
;; @0043                               v9 = func_addr.i64 fn0
;; @0043                               store notrap aligned region5 v9, v0+92
;; @0043                               store notrap aligned region6 v0, v0+108
;; @0043                               v11 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0043                               v12 = load.i32 notrap aligned readonly can_move region3 v11
;; @0043                               store notrap aligned region7 v12, v0+100
;; @0043                               v13 = iconst.i64 84
;; @0043                               v14 = iadd v0, v13  ; v13 = 84
;; @0045                               trapz v14, user16
;; @0045                               v15 = iconst.i32 0
;; @0045                               v18 = call fn2(v0, v14, v15, v15, v15)  ; v15 = 0, v15 = 0, v15 = 0
;; @0045                               v19 = load.i64 notrap aligned region8 v18+104
;; @004e                               jump block3
;;
;;                                 block3:
;; @0045                               v21 = uextend.i128 v19
;; @0040                               v5 = iconst.i64 64
;;                                     v158 = ishl v21, v5  ; v5 = 64
;;                                     v160 = ireduce.i64 v158
;;                                     v162 = bor v160, v18
;; @004e                               trapz v162, user16
;; @004e                               v31 = load.i64 notrap aligned region8 v162+104
;; @0045                               v20 = uextend.i128 v18
;; @0045                               v25 = bor v158, v20
;;                                     v164 = ushr v25, v5  ; v5 = 64
;; @004e                               v30 = ireduce.i64 v164
;; @004e                               v32 = icmp eq v31, v30
;; @004e                               trapz v32, user23
;; @004e                               v33 = iconst.i64 1
;; @004e                               v34 = iadd v31, v33  ; v33 = 1
;; @004e                               store notrap aligned region8 v34, v162+104
;; @004e                               v35 = load.i64 notrap aligned region9 v162+112
;; @004e                               v36 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v37 = load.i64 notrap aligned region7 v36+88
;; @004e                               v38 = load.i64 notrap aligned region7 v36+96
;; @004e                               store notrap aligned region10 v37, v35+88
;; @004e                               store notrap aligned region10 v38, v35+96
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned region9 v2, v162+112  ; v2 = 0
;; @004e                               v40 = iconst.i64 2
;; @004e                               store notrap aligned region7 v40, v36+88  ; v40 = 2
;; @004e                               store notrap aligned region7 v162, v36+96
;; @004e                               v44 = iconst.i32 1
;; @004e                               store notrap aligned region11 v44, v162+32  ; v44 = 1
;; @004e                               v45 = iconst.i32 2
;; @004e                               store notrap aligned region11 v45, v38+32  ; v45 = 2
;; @004e                               v49 = load.i64 notrap aligned region12 v36+72
;; @004e                               v50 = load.i64 notrap aligned region13 v36+64
;; @004e                               v51 = load.i64 notrap aligned region14 v36+80
;; @004e                               store notrap aligned region15 v49, v38+8
;; @004e                               store notrap aligned region16 v50, v38+16
;; @004e                               store notrap aligned region17 v51, v38+24
;; @004e                               v52 = load.i64 notrap aligned region1 v36+24
;; @004e                               store notrap aligned region18 v52, v38
;; @004e                               v55 = load.i64 notrap aligned region18 v162
;; @004e                               store notrap aligned region1 v55, v36+24
;; @004e                               v56 = load.i64 notrap aligned region15 v162+8
;; @004e                               store notrap aligned region12 v56, v36+72
;; @004e                               v57 = load.i64 notrap aligned region16 v162+16
;; @004e                               store notrap aligned region13 v57, v36+64
;; @004e                               v58 = load.i64 notrap aligned region17 v162+24
;; @004e                               store notrap aligned region14 v58, v36+80
;; @004e                               v59 = iconst.i64 40
;; @004e                               v60 = iadd v38, v59  ; v59 = 40
;; @004e                               store notrap aligned region19 v44, v60+4  ; v44 = 1
;; @004e                               v62 = stack_addr.i64 ss0
;; @004e                               store notrap aligned region20 v62, v60+8
;; @004e                               v63 = iconst.i64 48
;; @004e                               v64 = iadd.i64 v0, v63  ; v63 = 48
;; @004e                               store notrap aligned region21 v64, v62
;; @004e                               store notrap aligned region22 v44, v60  ; v44 = 1
;; @004e                               store notrap aligned region17 v44, v38+56  ; v44 = 1
;; @004e                               v71 = iconst.i64 120
;; @004e                               v72 = iadd v35, v71  ; v71 = 120
;; @004e                               v73 = load.i64 notrap aligned region23 v72
;; @004e                               v74 = iconst.i64 -24
;; @004e                               v75 = iadd v73, v74  ; v74 = -24
;;                                     v166 = iconst.i64 0x0001_0000_0000
;; @004e                               v76 = stack_switch v75, v75, v166  ; v166 = 0x0001_0000_0000
;; @004e                               v78 = load.i64 notrap aligned region7 v36+88
;; @004e                               v79 = load.i64 notrap aligned region7 v36+96
;; @004e                               store notrap aligned region7 v37, v36+88
;; @004e                               store notrap aligned region7 v38, v36+96
;; @004e                               store notrap aligned region11 v44, v38+32  ; v44 = 1
;;                                     v169 = iconst.i32 0
;; @004e                               store notrap aligned region22 v169, v60  ; v169 = 0
;; @004e                               store notrap aligned region19 v169, v60+4  ; v169 = 0
;; @004e                               store notrap aligned region20 v2, v60+8  ; v2 = 0
;; @004e                               store notrap aligned region17 v2, v38+56  ; v2 = 0
;; @004e                               brif v76, block7, block4
;;
;;                                 block7:
;; @004e                               v69 = iconst.i64 32
;; @004e                               v86 = ushr.i64 v76, v69  ; v69 = 32
;; @004e                               v87 = iconst.i64 4
;; @004e                               v88 = icmp eq v86, v87  ; v87 = 4
;; @004e                               brif v88, block6, block5
;;
;;                                 block6 cold:
;; @004e                               v91 = iconst.i32 5
;; @004e                               store notrap aligned region11 v91, v79+32  ; v91 = 5
;; @004e                               v94 = load.i64 notrap aligned region18 v38
;; @004e                               store notrap aligned region1 v94, v36+24
;; @004e                               v95 = load.i64 notrap aligned region15 v38+8
;; @004e                               store notrap aligned region12 v95, v36+72
;; @004e                               v96 = load.i64 notrap aligned region16 v38+16
;; @004e                               store notrap aligned region13 v96, v36+64
;; @004e                               v97 = load.i64 notrap aligned region17 v38+24
;; @004e                               store notrap aligned region14 v97, v36+80
;;                                     v178 = iconst.i32 0
;;                                     v179 = iconst.i64 144
;;                                     v180 = iadd.i64 v79, v179  ; v179 = 144
;; @004e                               store notrap aligned region22 v178, v180  ; v178 = 0
;; @004e                               store notrap aligned region19 v178, v180+4  ; v178 = 0
;;                                     v181 = iconst.i64 0
;; @004e                               store notrap aligned region20 v181, v180+8  ; v181 = 0
;;                                     v182 = iconst.i64 168
;;                                     v183 = iadd.i64 v79, v182  ; v182 = 168
;; @004e                               store notrap aligned region22 v178, v183  ; v178 = 0
;; @004e                               store notrap aligned region19 v178, v183+4  ; v178 = 0
;; @004e                               store notrap aligned region20 v181, v183+8  ; v181 = 0
;; @004e                               try_call fn3(v0), sig3, block9, [ context v0 ]
;;
;;                                 block9:
;; @004e                               trap user12
;;
;;                                 block5:
;; @004e                               v112 = load.i64 notrap aligned region12 v36+72
;; @004e                               v113 = load.i64 notrap aligned region13 v36+64
;; @004e                               v114 = load.i64 notrap aligned region14 v36+80
;; @004e                               store notrap aligned region15 v112, v79+8
;; @004e                               store notrap aligned region16 v113, v79+16
;; @004e                               store notrap aligned region17 v114, v79+24
;; @004e                               v117 = load.i64 notrap aligned region18 v38
;; @004e                               store notrap aligned region1 v117, v36+24
;; @004e                               v118 = load.i64 notrap aligned region15 v38+8
;; @004e                               store notrap aligned region12 v118, v36+72
;; @004e                               v119 = load.i64 notrap aligned region16 v38+16
;; @004e                               store notrap aligned region13 v119, v36+64
;; @004e                               v120 = load.i64 notrap aligned region17 v38+24
;; @004e                               store notrap aligned region14 v120, v36+80
;; @004e                               v122 = load.i64 notrap aligned region8 v79+104
;; @004e                               jump block8
;;
;;                                 block10 cold:
;; @004e                               trap user12
;;
;;                                 block11:
;; @004e                               v129 = iconst.i64 168
;; @004e                               v130 = iadd.i64 v79, v129  ; v129 = 168
;; @004e                               v131 = load.i64 notrap aligned region20 v130+8
;;                                     v175 = iconst.i32 0
;; @004e                               store notrap aligned region22 v175, v130  ; v175 = 0
;; @004e                               v124 = uextend.i128 v122
;;                                     v176 = iconst.i64 64
;;                                     v177 = ishl v124, v176  ; v176 = 64
;; @004e                               v123 = uextend.i128 v79
;; @004e                               v128 = bor v177, v123
;; @004e                               jump block2(v128)
;;
;;                                 block8:
;; @004e                               v121 = ireduce.i32 v76
;; @004e                               br_table v121, block10, [block11]
;;
;;                                 block4:
;; @004e                               v135 = load.i64 notrap aligned region18 v38
;; @004e                               store notrap aligned region1 v135, v36+24
;; @004e                               v136 = load.i64 notrap aligned region15 v38+8
;; @004e                               store notrap aligned region12 v136, v36+72
;; @004e                               v137 = load.i64 notrap aligned region16 v38+16
;; @004e                               store notrap aligned region13 v137, v36+64
;; @004e                               v138 = load.i64 notrap aligned region17 v38+24
;; @004e                               store notrap aligned region14 v138, v36+80
;; @004e                               v141 = iconst.i32 4
;; @004e                               store notrap aligned region11 v141, v79+32  ; v141 = 4
;; @004e                               v142 = iconst.i64 144
;; @004e                               v143 = iadd.i64 v79, v142  ; v142 = 144
;; @004e                               v144 = load.i64 notrap aligned region20 v143+8
;;                                     v170 = iconst.i32 0
;; @004e                               store notrap aligned region22 v170, v143  ; v170 = 0
;; @004e                               store notrap aligned region19 v170, v143+4  ; v170 = 0
;;                                     v171 = iconst.i64 0
;; @004e                               store notrap aligned region20 v171, v143+8  ; v171 = 0
;;                                     v172 = uextend.i128 v171  ; v171 = 0
;;                                     v173 = iconst.i64 64
;;                                     v174 = ishl v172, v173  ; v173 = 64
;; @0040                               v8 = bor v174, v172
;; @0056                               jump block2(v8)
;;
;;                                 block2(v155: i128):
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
