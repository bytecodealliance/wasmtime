;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wfuel=100'

(module
  (memory 1)
  (func $copy_16 (param i32 i32)
    (memory.copy (local.get 0) (local.get 1) (i32.const 16))
  )
  (func $fill_128 (param i32)
    (memory.fill (local.get 0) (i32.const 0) (i32.const 128))
  )
  (func $fill_4096 (param i32)
    (memory.fill (local.get 0) (i32.const 0) (i32.const 4096))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:12 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0023                               v4 = load.i64 notrap aligned readonly can_move v0+8
;; @0023                               v5 = load.i64 notrap aligned v4
;; @0023                               v6 = iconst.i64 1
;; @0023                               v7 = iadd v5, v6  ; v6 = 1
;; @0023                               v8 = iconst.i64 0
;; @0023                               v9 = icmp sge v7, v8  ; v8 = 0
;; @0023                               brif v9, block2, block3(v7)
;;
;;                                 block2:
;;                                     v67 = iadd.i64 v5, v6  ; v6 = 1
;; @0023                               store notrap aligned v67, v4
;; @0023                               v12 = call fn0(v0)
;; @0023                               v14 = load.i64 notrap aligned v4
;; @0023                               jump block3(v14)
;;
;;                                 block3(v46: i64):
;; @002a                               v19 = load.i64 notrap aligned v0+64
;; @002a                               v20 = uextend.i64 v2
;;                                     v56 = iconst.i64 16
;; @002a                               v24 = iadd v20, v56  ; v56 = 16
;; @002a                               v25 = icmp ugt v24, v19
;; @002a                               trapnz v25, heap_oob
;; @002a                               v33 = uextend.i64 v3
;; @002a                               v37 = iadd v33, v56  ; v56 = 16
;; @002a                               v38 = icmp ugt v37, v19
;; @002a                               trapnz v38, heap_oob
;; @002a                               v26 = load.i64 notrap aligned readonly can_move v0+56
;; @002a                               v43 = iadd v26, v33
;; @002a                               v45 = load.i8x16 notrap aligned little region0 v43
;; @002a                               v30 = iadd v26, v20
;; @002a                               store notrap aligned little region0 v45, v30
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               v47 = iconst.i64 20
;; @002e                               v48 = iadd.i64 v46, v47  ; v47 = 20
;; @002e                               store notrap aligned v48, v4
;; @002e                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:12 sig0
;;     fn1 = colocated u805306368:2 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0030                               v3 = load.i64 notrap aligned readonly can_move v0+8
;; @0030                               v4 = load.i64 notrap aligned v3
;; @0030                               v5 = iconst.i64 1
;; @0030                               v6 = iadd v4, v5  ; v5 = 1
;; @0030                               v7 = iconst.i64 0
;; @0030                               v8 = icmp sge v6, v7  ; v7 = 0
;; @0030                               brif v8, block2, block3(v6)
;;
;;                                 block2:
;;                                     v51 = iadd.i64 v4, v5  ; v5 = 1
;; @0030                               store notrap aligned v51, v3
;; @0030                               v11 = call fn0(v0)
;; @0030                               v13 = load.i64 notrap aligned v3
;; @0030                               jump block3(v13)
;;
;;                                 block3(v32: i64):
;; @0038                               v18 = load.i64 notrap aligned v0+64
;; @0038                               v19 = uextend.i64 v2
;;                                     v41 = iconst.i64 128
;; @0038                               v23 = iadd v19, v41  ; v41 = 128
;; @0038                               v24 = icmp ugt v23, v18
;; @0038                               trapnz v24, heap_oob
;; @0038                               v25 = load.i64 notrap aligned readonly can_move v0+56
;; @0038                               v29 = iadd v25, v19
;; @0033                               v15 = iconst.i32 0
;; @0038                               call fn1(v0, v29, v15, v41)  ; v15 = 0, v41 = 128
;; @003b                               jump block1
;;
;;                                 block1:
;; @003b                               v33 = iconst.i64 132
;; @003b                               v34 = iadd.i64 v32, v33  ; v33 = 132
;; @003b                               store notrap aligned v34, v3
;; @003b                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:12 sig0
;;     fn1 = colocated u805306368:2 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003d                               v3 = load.i64 notrap aligned readonly can_move v0+8
;; @003d                               v4 = load.i64 notrap aligned v3
;; @003d                               v5 = iconst.i64 1
;; @003d                               v6 = iadd v4, v5  ; v5 = 1
;; @003d                               v7 = iconst.i64 0
;; @003d                               v8 = icmp sge v6, v7  ; v7 = 0
;; @003d                               brif v8, block2, block3(v6)
;;
;;                                 block2:
;;                                     v61 = iadd.i64 v4, v5  ; v5 = 1
;; @003d                               store notrap aligned v61, v3
;; @003d                               v11 = call fn0(v0)
;; @003d                               v13 = load.i64 notrap aligned v3
;; @003d                               jump block3(v13)
;;
;;                                 block3(v17: i64):
;; @0045                               v18 = iconst.i64 4100
;; @0045                               v19 = iadd v17, v18  ; v18 = 4100
;;                                     v62 = iconst.i64 0
;;                                     v63 = icmp sge v19, v62  ; v62 = 0
;; @0045                               brif v63, block4, block5(v19)
;;
;;                                 block4:
;;                                     v64 = iadd.i64 v17, v18  ; v18 = 4100
;; @0045                               store notrap aligned v64, v3
;; @0045                               v24 = call fn0(v0)
;; @0045                               v26 = load.i64 notrap aligned v3
;; @0045                               jump block5(v26)
;;
;;                                 block5(v43: i64):
;; @0045                               v28 = load.i64 notrap aligned v0+64
;; @0045                               v29 = uextend.i64 v2
;;                                     v51 = iconst.i64 4096
;; @0045                               v33 = iadd v29, v51  ; v51 = 4096
;; @0045                               v34 = icmp ugt v33, v28
;; @0045                               trapnz v34, heap_oob
;; @0045                               v35 = load.i64 notrap aligned readonly can_move v0+56
;; @0045                               v39 = iadd v35, v29
;; @0040                               v15 = iconst.i32 0
;; @0045                               call fn1(v0, v39, v15, v51)  ; v15 = 0, v51 = 4096
;; @0048                               jump block1
;;
;;                                 block1:
;; @0048                               store.i64 notrap aligned v43, v3
;; @0048                               return
;; }
