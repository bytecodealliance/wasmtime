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
;;                                     v72 = iadd.i64 v6, v7  ; v7 = 1
;; @001e                               store notrap aligned v72, v5
;; @001e                               v13 = call fn0(v0)
;; @001e                               v15 = load.i64 notrap aligned v5
;; @001e                               jump block3(v15)
;;
;;                                 block3(v20: i64):
;; @0025                               v21 = iconst.i64 4
;; @0025                               v22 = iadd v20, v21  ; v21 = 4
;; @0025                               v19 = uextend.i64 v4
;; @0025                               v23 = iadd v22, v19
;;                                     v73 = iconst.i64 0
;;                                     v74 = icmp sge v23, v73  ; v73 = 0
;; @0025                               brif v74, block4, block5(v23)
;;
;;                                 block4:
;; @0025                               store.i64 notrap aligned v23, v5
;; @0025                               v28 = call fn0(v0)
;; @0025                               v30 = load.i64 notrap aligned v5
;; @0025                               jump block5(v30)
;;
;;                                 block5(v60: i64):
;; @0025                               v32 = load.i64 notrap aligned v0+64
;; @0025                               v33 = uextend.i64 v2
;; @0025                               v37 = iadd v33, v19
;; @0025                               v38 = icmp ugt v37, v32
;; @0025                               trapnz v38, heap_oob
;; @0025                               v46 = uextend.i64 v3
;; @0025                               v50 = iadd v46, v19
;; @0025                               v51 = icmp ugt v50, v32
;; @0025                               trapnz v51, heap_oob
;; @0025                               v39 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v43 = iadd v39, v33
;; @0025                               v56 = iadd v39, v46
;; @0025                               call fn1(v0, v43, v56, v19)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               store.i64 notrap aligned v60, v5
;; @0029                               return
;; }
