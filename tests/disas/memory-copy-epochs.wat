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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:13 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v6 = load.i64 notrap aligned v0+24
;; @001e                               v7 = load.i64 notrap aligned v6
;; @001e                               v8 = load.i64 notrap aligned readonly can_move v0+8
;; @001e                               v9 = load.i64 notrap aligned v8+8
;; @001e                               v10 = icmp uge v7, v9
;; @001e                               brif v10, block3, block2(v9)
;;
;;                                 block3 cold:
;; @001e                               v12 = call fn0(v0)
;; @001e                               jump block2(v12)
;;
;;                                 block2(v18: i64):
;; @0025                               v17 = load.i64 notrap aligned v6
;; @0025                               v19 = icmp uge v17, v18
;; @0025                               brif v19, block5, block4
;;
;;                                 block5 cold:
;; @0025                               v21 = load.i64 notrap aligned v8+8
;; @0025                               v22 = icmp.i64 uge v17, v21
;; @0025                               brif v22, block6, block4
;;
;;                                 block6 cold:
;; @0025                               v24 = call fn0(v0)
;; @0025                               jump block4
;;
;;                                 block4:
;; @0025                               v26 = load.i64 notrap aligned v0+64
;; @0025                               v27 = uextend.i64 v2
;; @0025                               v28 = uextend.i64 v4
;; @0025                               v31 = iadd v27, v28
;; @0025                               v32 = icmp ugt v31, v26
;; @0025                               trapnz v32, heap_oob
;; @0025                               v40 = uextend.i64 v3
;; @0025                               v44 = iadd v40, v28
;; @0025                               v45 = icmp ugt v44, v26
;; @0025                               trapnz v45, heap_oob
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v37 = iadd v33, v27
;; @0025                               v50 = iadd v33, v40
;; @0025                               call fn1(v0, v37, v50, v28)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
