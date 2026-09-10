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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 106 ""
;;     region3 = 225 ""
;;     region4 = 214 ""
;;     region5 = 13 ""
;;     region6 = 55 ""
;;     region7 = 153 ""
;;     region8 = 118 ""
;;     region9 = 255 ""
;;     region10 = 211 ""
;;     region11 = 82 ""
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
;; @003b                               v28 = iconst.i64 144
;; @003b                               v29 = iadd.i64 v6, v28  ; v28 = 144
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
;;                                     v87 = iadd.i64 v6, v28  ; v28 = 144
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
;;                                     v84 = iadd.i64 v6, v28  ; v28 = 144
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
;;                                     v154 = ishl v17, v5  ; v5 = 64
;;                                     v156 = ireduce.i64 v154
;;                                     v158 = bor v156, v14
;; @004e                               trapz v158, user16
;; @004e                               v27 = load.i64 notrap aligned region2 v158+88
;; @0045                               v16 = uextend.i128 v14
;; @0045                               v21 = bor v154, v16
;;                                     v160 = ushr v21, v5  ; v5 = 64
;; @004e                               v26 = ireduce.i64 v160
;; @004e                               v28 = icmp eq v27, v26
;; @004e                               trapz v28, user23
;; @004e                               v29 = iconst.i64 1
;; @004e                               v30 = iadd v27, v29  ; v29 = 1
;; @004e                               store notrap aligned region2 v30, v158+88
;; @004e                               v31 = load.i64 notrap aligned region3 v158+80
;; @004e                               v32 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v33 = load.i64 notrap aligned region4 v32+88
;; @004e                               v34 = load.i64 notrap aligned region4 v32+96
;; @004e                               store notrap aligned region5 v33, v31+64
;; @004e                               store notrap aligned region5 v34, v31+72
;; @0040                               v2 = iconst.i64 0
;; @004e                               store notrap aligned region3 v2, v158+80  ; v2 = 0
;; @004e                               v36 = iconst.i64 2
;; @004e                               store notrap aligned region4 v36, v32+88  ; v36 = 2
;; @004e                               store notrap aligned region4 v158, v32+96
;; @004e                               v40 = iconst.i32 1
;; @004e                               store notrap aligned region6 v40, v158+32  ; v40 = 1
;; @004e                               v41 = iconst.i32 2
;; @004e                               store notrap aligned region6 v41, v34+32  ; v41 = 2
;; @004e                               v45 = load.i64 notrap aligned region7 v32+72
;; @004e                               v46 = load.i64 notrap aligned region8 v32+64
;; @004e                               v47 = load.i64 notrap aligned region9 v32+80
;; @004e                               store notrap aligned region10 v45, v34+8
;; @004e                               store notrap aligned region11 v46, v34+16
;; @004e                               store notrap aligned region12 v47, v34+24
;; @004e                               v48 = load.i64 notrap aligned region1 v32+24
;; @004e                               store notrap aligned region13 v48, v34
;; @004e                               v51 = load.i64 notrap aligned region13 v158
;; @004e                               store notrap aligned region1 v51, v32+24
;; @004e                               v52 = load.i64 notrap aligned region10 v158+8
;; @004e                               store notrap aligned region7 v52, v32+72
;; @004e                               v53 = load.i64 notrap aligned region11 v158+16
;; @004e                               store notrap aligned region8 v53, v32+64
;; @004e                               v54 = load.i64 notrap aligned region12 v158+24
;; @004e                               store notrap aligned region9 v54, v32+80
;; @004e                               v55 = iconst.i64 40
;; @004e                               v56 = iadd v34, v55  ; v55 = 40
;; @004e                               store notrap aligned region14 v40, v56+4  ; v40 = 1
;; @004e                               v58 = stack_addr.i64 ss0
;; @004e                               store notrap aligned region15 v58, v56+8
;; @004e                               v59 = iconst.i64 48
;; @004e                               v60 = iadd.i64 v0, v59  ; v59 = 48
;; @004e                               store notrap aligned region16 v60, v58
;; @004e                               store notrap aligned region17 v40, v56  ; v40 = 1
;; @004e                               store notrap aligned region12 v40, v34+56  ; v40 = 1
;; @004e                               v67 = iconst.i64 96
;; @004e                               v68 = iadd v31, v67  ; v67 = 96
;; @004e                               v69 = load.i64 notrap aligned region18 v68
;; @004e                               v70 = iconst.i64 -24
;; @004e                               v71 = iadd v69, v70  ; v70 = -24
;;                                     v162 = iconst.i64 0x0001_0000_0000
;; @004e                               v72 = stack_switch v71, v71, v162  ; v162 = 0x0001_0000_0000
;; @004e                               v74 = load.i64 notrap aligned region4 v32+88
;; @004e                               v75 = load.i64 notrap aligned region4 v32+96
;; @004e                               store notrap aligned region4 v33, v32+88
;; @004e                               store notrap aligned region4 v34, v32+96
;; @004e                               store notrap aligned region6 v40, v34+32  ; v40 = 1
;;                                     v165 = iconst.i32 0
;; @004e                               store notrap aligned region17 v165, v56  ; v165 = 0
;; @004e                               store notrap aligned region14 v165, v56+4  ; v165 = 0
;; @004e                               store notrap aligned region15 v2, v56+8  ; v2 = 0
;; @004e                               store notrap aligned region12 v2, v34+56  ; v2 = 0
;; @004e                               brif v72, block7, block4
;;
;;                                 block7:
;; @004e                               v65 = iconst.i64 32
;; @004e                               v82 = ushr.i64 v72, v65  ; v65 = 32
;; @004e                               v83 = iconst.i64 4
;; @004e                               v84 = icmp eq v82, v83  ; v83 = 4
;; @004e                               brif v84, block6, block5
;;
;;                                 block6 cold:
;; @004e                               v87 = iconst.i32 5
;; @004e                               store notrap aligned region6 v87, v75+32  ; v87 = 5
;; @004e                               v90 = load.i64 notrap aligned region13 v34
;; @004e                               store notrap aligned region1 v90, v32+24
;; @004e                               v91 = load.i64 notrap aligned region10 v34+8
;; @004e                               store notrap aligned region7 v91, v32+72
;; @004e                               v92 = load.i64 notrap aligned region11 v34+16
;; @004e                               store notrap aligned region8 v92, v32+64
;; @004e                               v93 = load.i64 notrap aligned region12 v34+24
;; @004e                               store notrap aligned region9 v93, v32+80
;;                                     v174 = iconst.i32 0
;;                                     v175 = iconst.i64 120
;;                                     v176 = iadd.i64 v75, v175  ; v175 = 120
;; @004e                               store notrap aligned region17 v174, v176  ; v174 = 0
;; @004e                               store notrap aligned region14 v174, v176+4  ; v174 = 0
;;                                     v177 = iconst.i64 0
;; @004e                               store notrap aligned region15 v177, v176+8  ; v177 = 0
;;                                     v178 = iconst.i64 144
;;                                     v179 = iadd.i64 v75, v178  ; v178 = 144
;; @004e                               store notrap aligned region17 v174, v179  ; v174 = 0
;; @004e                               store notrap aligned region14 v174, v179+4  ; v174 = 0
;; @004e                               store notrap aligned region15 v177, v179+8  ; v177 = 0
;; @004e                               try_call fn2(v0), sig2, block9, [ context v0 ]
;;
;;                                 block9:
;; @004e                               trap user12
;;
;;                                 block5:
;; @004e                               v108 = load.i64 notrap aligned region7 v32+72
;; @004e                               v109 = load.i64 notrap aligned region8 v32+64
;; @004e                               v110 = load.i64 notrap aligned region9 v32+80
;; @004e                               store notrap aligned region10 v108, v75+8
;; @004e                               store notrap aligned region11 v109, v75+16
;; @004e                               store notrap aligned region12 v110, v75+24
;; @004e                               v113 = load.i64 notrap aligned region13 v34
;; @004e                               store notrap aligned region1 v113, v32+24
;; @004e                               v114 = load.i64 notrap aligned region10 v34+8
;; @004e                               store notrap aligned region7 v114, v32+72
;; @004e                               v115 = load.i64 notrap aligned region11 v34+16
;; @004e                               store notrap aligned region8 v115, v32+64
;; @004e                               v116 = load.i64 notrap aligned region12 v34+24
;; @004e                               store notrap aligned region9 v116, v32+80
;; @004e                               v118 = load.i64 notrap aligned region2 v75+88
;; @004e                               jump block8
;;
;;                                 block10 cold:
;; @004e                               trap user12
;;
;;                                 block11:
;; @004e                               v125 = iconst.i64 144
;; @004e                               v126 = iadd.i64 v75, v125  ; v125 = 144
;; @004e                               v127 = load.i64 notrap aligned region15 v126+8
;;                                     v171 = iconst.i32 0
;; @004e                               store notrap aligned region17 v171, v126  ; v171 = 0
;; @004e                               v120 = uextend.i128 v118
;;                                     v172 = iconst.i64 64
;;                                     v173 = ishl v120, v172  ; v172 = 64
;; @004e                               v119 = uextend.i128 v75
;; @004e                               v124 = bor v173, v119
;; @004e                               jump block2(v124)
;;
;;                                 block8:
;; @004e                               v117 = ireduce.i32 v72
;; @004e                               br_table v117, block10, [block11]
;;
;;                                 block4:
;; @004e                               v131 = load.i64 notrap aligned region13 v34
;; @004e                               store notrap aligned region1 v131, v32+24
;; @004e                               v132 = load.i64 notrap aligned region10 v34+8
;; @004e                               store notrap aligned region7 v132, v32+72
;; @004e                               v133 = load.i64 notrap aligned region11 v34+16
;; @004e                               store notrap aligned region8 v133, v32+64
;; @004e                               v134 = load.i64 notrap aligned region12 v34+24
;; @004e                               store notrap aligned region9 v134, v32+80
;; @004e                               v137 = iconst.i32 4
;; @004e                               store notrap aligned region6 v137, v75+32  ; v137 = 4
;; @004e                               v138 = iconst.i64 120
;; @004e                               v139 = iadd.i64 v75, v138  ; v138 = 120
;; @004e                               v140 = load.i64 notrap aligned region15 v139+8
;;                                     v166 = iconst.i32 0
;; @004e                               store notrap aligned region17 v166, v139  ; v166 = 0
;; @004e                               store notrap aligned region14 v166, v139+4  ; v166 = 0
;;                                     v167 = iconst.i64 0
;; @004e                               store notrap aligned region15 v167, v139+8  ; v167 = 0
;;                                     v168 = uextend.i128 v167  ; v167 = 0
;;                                     v169 = iconst.i64 64
;;                                     v170 = ishl v168, v169  ; v169 = 64
;; @0040                               v8 = bor v170, v168
;; @0056                               jump block2(v8)
;;
;;                                 block2(v151: i128):
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
