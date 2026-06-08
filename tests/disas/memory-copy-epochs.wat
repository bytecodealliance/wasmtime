;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wepoch-interruption'

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
;;     gv4 = load.i64 notrap aligned gv3+64
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:13 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v5 = load.i64 notrap aligned v0+24
;; @001e                               v6 = load.i64 notrap aligned v5
;; @001e                               v7 = load.i64 notrap aligned readonly can_move v0+8
;; @001e                               v8 = load.i64 notrap aligned v7+8
;; @001e                               v9 = icmp uge v6, v8
;; @001e                               brif v9, block3, block2(v8)
;;
;;                                 block3 cold:
;; @001e                               v10 = call fn0(v0)
;; @001e                               jump block2(v10)
;;
;;                                 block2(v61: i64):
;; @0025                               v15 = load.i64 notrap aligned v0+64
;; @0025                               v16 = uextend.i64 v2
;; @0025                               v17 = uextend.i64 v4
;; @0025                               v20 = iadd v16, v17
;; @0025                               v21 = icmp ugt v20, v15
;; @0025                               trapnz v21, heap_oob
;; @0025                               v29 = uextend.i64 v3
;; @0025                               v33 = iadd v29, v17
;; @0025                               v34 = icmp ugt v33, v15
;; @0025                               trapnz v34, heap_oob
;; @0025                               v22 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v39 = iadd v22, v29
;; @0025                               v26 = iadd v22, v16
;; @0025                               v42 = icmp ugt v39, v26
;; @0025                               brif v42, block6, block7
;;
;;                                 block4(v44: i64, v45: i64, v46: i64, v49: i64):
;; @0025                               v48 = load.i64 notrap aligned v5
;; @0025                               v50 = icmp uge v48, v49
;; @0025                               brif v50, block9, block8(v49)
;;
;;                                 block5(v88: i64, v89: i64, v90: i64, v94: i64):
;; @0025                               v93 = load.i64 notrap aligned v5
;; @0025                               v96 = icmp uge v93, v94
;; @0025                               brif v96, block17, block16
;;
;;                                 block6:
;;                                     v112 = iconst.i64 0x0800_0000
;;                                     v113 = icmp.i64 ugt v17, v112  ; v112 = 0x0800_0000
;; @0025                               brif v113, block4(v26, v39, v17, v61), block5(v26, v39, v17, v61)
;;
;;                                 block9 cold:
;; @0025                               v52 = load.i64 notrap aligned v7+8
;; @0025                               v53 = icmp.i64 uge v48, v52
;; @0025                               brif v53, block10, block8(v52)
;;
;;                                 block10 cold:
;; @0025                               v54 = call fn0(v0)
;; @0025                               jump block8(v54)
;;
;;                                 block8(v62: i64):
;; @0025                               call fn1(v0, v44, v45, v112)  ; v112 = 0x0800_0000
;; @0025                               v57 = isub.i64 v46, v112  ; v112 = 0x0800_0000
;; @0025                               v58 = icmp ugt v57, v112  ; v112 = 0x0800_0000
;; @0025                               v55 = iadd.i64 v44, v112  ; v112 = 0x0800_0000
;; @0025                               v56 = iadd.i64 v45, v112  ; v112 = 0x0800_0000
;; @0025                               brif v58, block4(v55, v56, v57, v62), block5(v55, v56, v57, v62)
;;
;;                                 block7:
;; @0025                               v41 = iconst.i64 0x0800_0000
;; @0025                               v65 = icmp.i64 ugt v17, v41  ; v41 = 0x0800_0000
;; @0025                               v63 = iadd.i64 v26, v17
;; @0025                               v64 = iadd.i64 v39, v17
;; @0025                               brif v65, block11(v63, v64, v17, v61), block12(v63, v64, v17, v61)
;;
;;                                 block11(v66: i64, v67: i64, v68: i64, v73: i64):
;; @0025                               v72 = load.i64 notrap aligned v5
;; @0025                               v74 = icmp uge v72, v73
;; @0025                               brif v74, block14, block13(v73)
;;
;;                                 block14 cold:
;; @0025                               v76 = load.i64 notrap aligned v7+8
;; @0025                               v77 = icmp.i64 uge v72, v76
;; @0025                               brif v77, block15, block13(v76)
;;
;;                                 block15 cold:
;; @0025                               v78 = call fn0(v0)
;; @0025                               jump block13(v78)
;;
;;                                 block13(v82: i64):
;;                                     v107 = iconst.i64 0x0800_0000
;;                                     v108 = isub.i64 v66, v107  ; v107 = 0x0800_0000
;;                                     v109 = isub.i64 v67, v107  ; v107 = 0x0800_0000
;; @0025                               call fn1(v0, v108, v109, v107)  ; v107 = 0x0800_0000
;;                                     v110 = isub.i64 v68, v107  ; v107 = 0x0800_0000
;;                                     v111 = icmp ugt v110, v107  ; v107 = 0x0800_0000
;; @0025                               brif v111, block11(v108, v109, v110, v82), block12(v108, v109, v110, v82)
;;
;;                                 block12(v83: i64, v84: i64, v85: i64, v95: i64):
;; @0025                               v86 = isub v83, v85
;; @0025                               v87 = isub v84, v85
;; @0025                               jump block5(v86, v87, v85, v95)
;;
;;                                 block17 cold:
;; @0025                               v98 = load.i64 notrap aligned v7+8
;; @0025                               v99 = icmp.i64 uge v93, v98
;; @0025                               brif v99, block18, block16
;;
;;                                 block18 cold:
;; @0025                               v100 = call fn0(v0)
;; @0025                               jump block16
;;
;;                                 block16:
;; @0025                               call fn1(v0, v88, v89, v90)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
