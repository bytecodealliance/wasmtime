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
;;     fn1 = colocated u805306368:3 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v5 = load.i64 notrap aligned readonly can_move v0+8
;; @001e                               v6 = load.i64 notrap aligned v5
;;                                     v87 = iconst.i64 1
;; @001e                               v7 = iadd v6, v87  ; v87 = 1
;; @001e                               v8 = iconst.i64 0
;; @001e                               v9 = icmp sge v7, v8  ; v8 = 0
;; @001e                               brif v9, block2, block3(v7)
;;
;;                                 block2:
;;                                     v91 = iadd.i64 v6, v87  ; v87 = 1
;; @001e                               store notrap aligned v91, v5
;; @001e                               v12 = call fn0(v0)
;; @001e                               v14 = load.i64 notrap aligned v5
;; @001e                               jump block3(v14)
;;
;;                                 block3(v40: i64):
;; @0025                               v19 = load.i64 notrap aligned v0+64
;; @0025                               v20 = uextend.i64 v2
;; @0025                               v21 = uextend.i64 v4
;; @0025                               v22 = iadd v20, v21
;; @0025                               v23 = icmp ugt v22, v19
;; @0025                               trapnz v23, heap_oob
;; @0025                               v30 = uextend.i64 v3
;; @0025                               v32 = iadd v30, v21
;; @0025                               v33 = icmp ugt v32, v19
;; @0025                               trapnz v33, heap_oob
;; @0025                               v42 = iconst.i64 0x0800_0000
;; @0025                               v43 = icmp ugt v21, v42  ; v42 = 0x0800_0000
;; @0025                               v24 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v27 = iadd v24, v20
;; @0025                               v37 = iadd v24, v30
;;                                     v80 = iconst.i64 4
;; @0025                               v41 = iadd v40, v80  ; v80 = 4
;; @0025                               brif v43, block4(v27, v37, v21, v41), block5(v27, v37, v21, v41)
;;
;;                                 block4(v44: i64, v45: i64, v46: i64, v47: i64):
;;                                     v92 = iconst.i64 0x0800_0000
;;                                     v93 = iadd v47, v92  ; v92 = 0x0800_0000
;;                                     v94 = iconst.i64 0
;;                                     v95 = icmp sge v93, v94  ; v94 = 0
;; @0025                               brif v95, block6, block7(v93)
;;
;;                                 block5(v60: i64, v61: i64, v62: i64, v63: i64):
;; @0025                               v64 = iadd v63, v62
;;                                     v101 = iconst.i64 0
;;                                     v102 = icmp sge v64, v101  ; v101 = 0
;; @0025                               brif v102, block8, block9(v64)
;;
;;                                 block6:
;; @0025                               store.i64 notrap aligned v93, v5
;; @0025                               v53 = call fn0(v0)
;; @0025                               v55 = load.i64 notrap aligned v5
;; @0025                               jump block7(v55)
;;
;;                                 block7(v72: i64):
;;                                     v96 = iconst.i64 0x0800_0000
;; @0025                               call fn1(v0, v44, v45, v96)  ; v96 = 0x0800_0000
;;                                     v97 = isub.i64 v46, v96  ; v96 = 0x0800_0000
;;                                     v98 = icmp ugt v97, v96  ; v96 = 0x0800_0000
;;                                     v99 = iadd.i64 v44, v96  ; v96 = 0x0800_0000
;;                                     v100 = iadd.i64 v45, v96  ; v96 = 0x0800_0000
;; @0025                               brif v98, block4(v99, v100, v97, v72), block5(v99, v100, v97, v72)
;;
;;                                 block8:
;; @0025                               store.i64 notrap aligned v64, v5
;; @0025                               v69 = call fn0(v0)
;; @0025                               v71 = load.i64 notrap aligned v5
;; @0025                               jump block9(v71)
;;
;;                                 block9(v74: i64):
;; @0025                               call fn1(v0, v60, v61, v62)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               store.i64 notrap aligned v74, v5
;; @0029                               return
;; }
