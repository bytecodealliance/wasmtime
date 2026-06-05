;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wfuel=100'

(module
  (memory 1)
  (func $copy (param i32 i32 i32)
    (memory.copy (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:12 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v5 = load.i64 notrap aligned readonly can_move v0+8
;; @001e                               v6 = load.i64 notrap aligned v5
;; @001e                               v7 = iconst.i64 1
;; @001e                               v8 = iadd v6, v7  ; v7 = 1
;; @001e                               v9 = iconst.i64 0
;; @001e                               v10 = icmp sge v8, v9  ; v9 = 0
;; @001e                               brif v10, block2, block3(v8)
;;
;;                                 block2:
;;                                     v125 = iadd.i64 v6, v7  ; v7 = 1
;; @001e                               store notrap aligned v125, v5
;; @001e                               v13 = call fn0(v0)
;; @001e                               v15 = load.i64 notrap aligned v5
;; @001e                               jump block3(v15)
;;
;;                                 block3(v47: i64):
;; @0025                               v20 = load.i64 notrap aligned v0+64
;; @0025                               v21 = uextend.i64 v2
;; @0025                               v22 = uextend.i64 v4
;; @0025                               v25 = iadd v21, v22
;; @0025                               v26 = icmp ugt v25, v20
;; @0025                               trapnz v26, heap_oob
;; @0025                               v34 = uextend.i64 v3
;; @0025                               v38 = iadd v34, v22
;; @0025                               v39 = icmp ugt v38, v20
;; @0025                               trapnz v39, heap_oob
;; @0025                               v27 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v44 = iadd v27, v34
;; @0025                               v31 = iadd v27, v21
;; @0025                               v51 = icmp ugt v44, v31
;; @0025                               brif v51, block6, block7
;;
;;                                 block4(v53: i64, v54: i64, v55: i64, v56: i64):
;; @0025                               v57 = iadd v56, v135  ; v135 = 0x0800_0000
;;                                     v139 = iconst.i64 0
;;                                     v140 = icmp sge v57, v139  ; v139 = 0
;; @0025                               brif v140, block8, block9(v57)
;;
;;                                 block5(v95: i64, v96: i64, v97: i64, v98: i64):
;; @0025                               v100 = iadd v98, v97
;;                                     v142 = iconst.i64 0
;;                                     v143 = icmp sge v100, v142  ; v142 = 0
;; @0025                               brif v143, block14, block15(v100)
;;
;;                                 block6:
;;                                     v135 = iconst.i64 0x0800_0000
;;                                     v136 = icmp.i64 ugt v22, v135  ; v135 = 0x0800_0000
;;                                     v137 = iconst.i64 4
;;                                     v138 = iadd.i64 v47, v137  ; v137 = 4
;; @0025                               brif v136, block4(v31, v44, v22, v138), block5(v31, v44, v22, v138)
;;
;;                                 block8:
;;                                     v141 = iadd.i64 v56, v135  ; v135 = 0x0800_0000
;; @0025                               store notrap aligned v141, v5
;; @0025                               v62 = call fn0(v0)
;; @0025                               v64 = load.i64 notrap aligned v5
;; @0025                               jump block9(v64)
;;
;;                                 block9(v69: i64):
;; @0025                               call fn1(v0, v53, v54, v135)  ; v135 = 0x0800_0000
;; @0025                               v67 = isub.i64 v55, v135  ; v135 = 0x0800_0000
;; @0025                               v68 = icmp ugt v67, v135  ; v135 = 0x0800_0000
;; @0025                               v65 = iadd.i64 v53, v135  ; v135 = 0x0800_0000
;; @0025                               v66 = iadd.i64 v54, v135  ; v135 = 0x0800_0000
;; @0025                               brif v68, block4(v65, v66, v67, v69), block5(v65, v66, v67, v69)
;;
;;                                 block7:
;; @0025                               v50 = iconst.i64 0x0800_0000
;; @0025                               v72 = icmp.i64 ugt v22, v50  ; v50 = 0x0800_0000
;; @0025                               v70 = iadd.i64 v31, v22
;; @0025                               v71 = iadd.i64 v44, v22
;; @0025                               v48 = iconst.i64 4
;; @0025                               v49 = iadd.i64 v47, v48  ; v48 = 4
;; @0025                               brif v72, block10(v70, v71, v22, v49), block11(v70, v71, v22, v49)
;;
;;                                 block10(v73: i64, v74: i64, v75: i64, v78: i64):
;;                                     v126 = iconst.i64 0x0800_0000
;;                                     v127 = iadd v78, v126  ; v126 = 0x0800_0000
;;                                     v128 = iconst.i64 0
;;                                     v129 = icmp sge v127, v128  ; v128 = 0
;; @0025                               brif v129, block12, block13(v127)
;;
;;                                 block12:
;; @0025                               store.i64 notrap aligned v127, v5
;; @0025                               v84 = call fn0(v0)
;; @0025                               v86 = load.i64 notrap aligned v5
;; @0025                               jump block13(v86)
;;
;;                                 block13(v89: i64):
;;                                     v130 = iconst.i64 0x0800_0000
;;                                     v131 = isub.i64 v73, v130  ; v130 = 0x0800_0000
;;                                     v132 = isub.i64 v74, v130  ; v130 = 0x0800_0000
;; @0025                               call fn1(v0, v131, v132, v130)  ; v130 = 0x0800_0000
;;                                     v133 = isub.i64 v75, v130  ; v130 = 0x0800_0000
;;                                     v134 = icmp ugt v133, v130  ; v130 = 0x0800_0000
;; @0025                               brif v134, block10(v131, v132, v133, v89), block11(v131, v132, v133, v89)
;;
;;                                 block11(v90: i64, v91: i64, v92: i64, v99: i64):
;; @0025                               v93 = isub v90, v92
;; @0025                               v94 = isub v91, v92
;; @0025                               jump block5(v93, v94, v92, v99)
;;
;;                                 block14:
;; @0025                               store.i64 notrap aligned v100, v5
;; @0025                               v105 = call fn0(v0)
;; @0025                               v107 = load.i64 notrap aligned v5
;; @0025                               jump block15(v107)
;;
;;                                 block15(v109: i64):
;; @0025                               call fn1(v0, v95, v96, v97)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               store.i64 notrap aligned v109, v5
;; @0029                               return
;; }
