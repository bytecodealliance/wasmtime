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
;;                                     v120 = iadd.i64 v6, v7  ; v7 = 1
;; @001e                               store notrap aligned v120, v5
;; @001e                               v12 = call fn0(v0)
;; @001e                               v14 = load.i64 notrap aligned v5
;; @001e                               jump block3(v14)
;;
;;                                 block3(v45: i64):
;; @0025                               v19 = load.i64 notrap aligned v0+64
;; @0025                               v20 = uextend.i64 v2
;; @0025                               v21 = uextend.i64 v4
;; @0025                               v24 = iadd v20, v21
;; @0025                               v25 = icmp ugt v24, v19
;; @0025                               trapnz v25, heap_oob
;; @0025                               v33 = uextend.i64 v3
;; @0025                               v37 = iadd v33, v21
;; @0025                               v38 = icmp ugt v37, v19
;; @0025                               trapnz v38, heap_oob
;; @0025                               v26 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v43 = iadd v26, v33
;; @0025                               v30 = iadd v26, v20
;; @0025                               v49 = icmp ugt v43, v30
;; @0025                               brif v49, block6, block7
;;
;;                                 block4(v51: i64, v52: i64, v53: i64, v54: i64):
;; @0025                               v55 = iadd v54, v130  ; v130 = 0x0800_0000
;;                                     v134 = iconst.i64 0
;;                                     v135 = icmp sge v55, v134  ; v134 = 0
;; @0025                               brif v135, block8, block9(v55)
;;
;;                                 block5(v91: i64, v92: i64, v93: i64, v94: i64):
;; @0025                               v96 = iadd v94, v93
;;                                     v137 = iconst.i64 0
;;                                     v138 = icmp sge v96, v137  ; v137 = 0
;; @0025                               brif v138, block14, block15(v96)
;;
;;                                 block6:
;;                                     v130 = iconst.i64 0x0800_0000
;;                                     v131 = icmp.i64 ugt v21, v130  ; v130 = 0x0800_0000
;;                                     v132 = iconst.i64 4
;;                                     v133 = iadd.i64 v45, v132  ; v132 = 4
;; @0025                               brif v131, block4(v30, v43, v21, v133), block5(v30, v43, v21, v133)
;;
;;                                 block8:
;;                                     v136 = iadd.i64 v54, v130  ; v130 = 0x0800_0000
;; @0025                               store notrap aligned v136, v5
;; @0025                               v59 = call fn0(v0)
;; @0025                               v61 = load.i64 notrap aligned v5
;; @0025                               jump block9(v61)
;;
;;                                 block9(v66: i64):
;; @0025                               call fn1(v0, v51, v52, v130)  ; v130 = 0x0800_0000
;; @0025                               v64 = isub.i64 v53, v130  ; v130 = 0x0800_0000
;; @0025                               v65 = icmp ugt v64, v130  ; v130 = 0x0800_0000
;; @0025                               v62 = iadd.i64 v51, v130  ; v130 = 0x0800_0000
;; @0025                               v63 = iadd.i64 v52, v130  ; v130 = 0x0800_0000
;; @0025                               brif v65, block4(v62, v63, v64, v66), block5(v62, v63, v64, v66)
;;
;;                                 block7:
;; @0025                               v48 = iconst.i64 0x0800_0000
;; @0025                               v69 = icmp.i64 ugt v21, v48  ; v48 = 0x0800_0000
;; @0025                               v67 = iadd.i64 v30, v21
;; @0025                               v68 = iadd.i64 v43, v21
;; @0025                               v46 = iconst.i64 4
;; @0025                               v47 = iadd.i64 v45, v46  ; v46 = 4
;; @0025                               brif v69, block10(v67, v68, v21, v47), block11(v67, v68, v21, v47)
;;
;;                                 block10(v70: i64, v71: i64, v72: i64, v75: i64):
;;                                     v121 = iconst.i64 0x0800_0000
;;                                     v122 = iadd v75, v121  ; v121 = 0x0800_0000
;;                                     v123 = iconst.i64 0
;;                                     v124 = icmp sge v122, v123  ; v123 = 0
;; @0025                               brif v124, block12, block13(v122)
;;
;;                                 block12:
;; @0025                               store.i64 notrap aligned v122, v5
;; @0025                               v80 = call fn0(v0)
;; @0025                               v82 = load.i64 notrap aligned v5
;; @0025                               jump block13(v82)
;;
;;                                 block13(v85: i64):
;;                                     v125 = iconst.i64 0x0800_0000
;;                                     v126 = isub.i64 v70, v125  ; v125 = 0x0800_0000
;;                                     v127 = isub.i64 v71, v125  ; v125 = 0x0800_0000
;; @0025                               call fn1(v0, v126, v127, v125)  ; v125 = 0x0800_0000
;;                                     v128 = isub.i64 v72, v125  ; v125 = 0x0800_0000
;;                                     v129 = icmp ugt v128, v125  ; v125 = 0x0800_0000
;; @0025                               brif v129, block10(v126, v127, v128, v85), block11(v126, v127, v128, v85)
;;
;;                                 block11(v86: i64, v87: i64, v88: i64, v95: i64):
;; @0025                               v89 = isub v86, v88
;; @0025                               v90 = isub v87, v88
;; @0025                               jump block5(v89, v90, v88, v95)
;;
;;                                 block14:
;; @0025                               store.i64 notrap aligned v96, v5
;; @0025                               v100 = call fn0(v0)
;; @0025                               v102 = load.i64 notrap aligned v5
;; @0025                               jump block15(v102)
;;
;;                                 block15(v104: i64):
;; @0025                               call fn1(v0, v91, v92, v93)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               store.i64 notrap aligned v104, v5
;; @0029                               return
;; }
