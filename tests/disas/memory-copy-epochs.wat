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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 24 "VMContext+0x18"
;;     region3 = 1744830464 "EpochCounter+0x0"
;;     region4 = 67108872 "VMStoreContext+0x8"
;;     region5 = 603979776 "VMMemoryDefinition+0x0"
;;     region6 = 603979784 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:13 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v5 = load.i64 notrap aligned region2 v0+24
;; @001e                               v6 = load.i64 notrap aligned region3 v5
;; @001e                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001e                               v8 = load.i64 notrap aligned region4 v7+8
;; @001e                               v9 = icmp uge v6, v8
;; @001e                               brif v9, block3, block2(v8)
;;
;;                                 block3 cold:
;; @001e                               v10 = call fn0(v0)
;; @001e                               jump block2(v10)
;;
;;                                 block2(v41: i64):
;; @0025                               v14 = load.i64 notrap aligned region6 v0+64
;; @0025                               v15 = uextend.i64 v2
;; @0025                               v16 = uextend.i64 v4
;; @0025                               v19 = iadd v15, v16
;; @0025                               v20 = icmp ugt v19, v14
;; @0025                               trapnz v20, heap_oob
;; @0025                               v27 = uextend.i64 v3
;; @0025                               v31 = iadd v27, v16
;; @0025                               v32 = icmp ugt v31, v14
;; @0025                               trapnz v32, heap_oob
;; @0025                               v21 = load.i64 notrap aligned readonly can_move region5 v0+56
;; @0025                               v25 = iadd v21, v15
;; @0025                               v37 = iadd v21, v27
;; @0025                               call fn1(v0, v25, v37, v16)
;; @0025                               v40 = load.i64 notrap aligned region3 v5
;; @0025                               v42 = icmp uge v40, v41
;; @0025                               brif v42, block5, block4
;;
;;                                 block5 cold:
;; @0025                               v44 = load.i64 notrap aligned region4 v7+8
;; @0025                               v45 = icmp.i64 uge v40, v44
;; @0025                               brif v45, block6, block4
;;
;;                                 block6 cold:
;; @0025                               v46 = call fn0(v0)
;; @0025                               jump block4
;;
;;                                 block4:
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
