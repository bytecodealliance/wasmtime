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
;;     fn0 = colocated u805306368:13 sig0
;;     fn1 = colocated u805306368:4 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v5 = load.i64 notrap aligned readonly can_move v0+8
;; @001e                               v6 = load.i64 notrap aligned v5
;;                                     v83 = iconst.i64 1
;; @001e                               v7 = iadd v6, v83  ; v83 = 1
;; @001e                               v8 = iconst.i64 0
;; @001e                               v9 = icmp sge v7, v8  ; v8 = 0
;; @001e                               brif v9, block2, block3(v7)
;;
;;                                 block2:
;;                                     v85 = iadd.i64 v6, v83  ; v83 = 1
;; @001e                               store notrap aligned v85, v5
;; @001e                               v12 = call fn0(v0)
;; @001e                               v14 = load.i64 notrap aligned v5
;; @001e                               jump block3(v14)
;;
;;                                 block3(v38: i64):
;; @0025                               v19 = load.i64 notrap aligned v0+64
;; @0025                               v20 = uextend.i64 v2
;; @0025                               v21 = uextend.i64 v4
;; @0025                               v22 = iadd v20, v21
;; @0025                               v23 = icmp ule v22, v19
;; @0025                               trapz v23, heap_oob
;; @0025                               v29 = uextend.i64 v3
;; @0025                               v31 = iadd v29, v21
;; @0025                               v32 = icmp ule v31, v19
;; @0025                               trapz v32, heap_oob
;; @0025                               v40 = iconst.i64 0x0800_0000
;; @0025                               v41 = icmp ugt v21, v40  ; v40 = 0x0800_0000
;; @0025                               v25 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v26 = iadd v25, v20
;; @0025                               v35 = iadd v25, v29
;;                                     v78 = iconst.i64 4
;; @0025                               v39 = iadd v38, v78  ; v78 = 4
;; @0025                               brif v41, block4(v26, v35, v21, v39), block5(v26, v35, v21, v39)
;;
;;                                 block4(v42: i64, v43: i64, v44: i64, v45: i64):
;;                                     v86 = iconst.i64 0x0800_0000
;;                                     v87 = iadd v45, v86  ; v86 = 0x0800_0000
;;                                     v88 = iconst.i64 0
;;                                     v89 = icmp sge v87, v88  ; v88 = 0
;; @0025                               brif v89, block6, block7(v87)
;;
;;                                 block5(v58: i64, v59: i64, v60: i64, v61: i64):
;; @0025                               v62 = iadd v61, v60
;;                                     v95 = iconst.i64 0
;;                                     v96 = icmp sge v62, v95  ; v95 = 0
;; @0025                               brif v96, block8, block9(v62)
;;
;;                                 block6:
;; @0025                               store.i64 notrap aligned v87, v5
;; @0025                               v51 = call fn0(v0)
;; @0025                               v53 = load.i64 notrap aligned v5
;; @0025                               jump block7(v53)
;;
;;                                 block7(v70: i64):
;;                                     v90 = iconst.i64 0x0800_0000
;; @0025                               call fn1(v0, v42, v43, v90)  ; v90 = 0x0800_0000
;;                                     v91 = isub.i64 v44, v90  ; v90 = 0x0800_0000
;;                                     v92 = icmp ugt v91, v90  ; v90 = 0x0800_0000
;;                                     v93 = iadd.i64 v42, v90  ; v90 = 0x0800_0000
;;                                     v94 = iadd.i64 v43, v90  ; v90 = 0x0800_0000
;; @0025                               brif v92, block4(v93, v94, v91, v70), block5(v93, v94, v91, v70)
;;
;;                                 block8:
;; @0025                               store.i64 notrap aligned v62, v5
;; @0025                               v67 = call fn0(v0)
;; @0025                               v69 = load.i64 notrap aligned v5
;; @0025                               jump block9(v69)
;;
;;                                 block9(v72: i64):
;; @0025                               call fn1(v0, v58, v59, v60)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               store.i64 notrap aligned v72, v5
;; @0029                               return
;; }
