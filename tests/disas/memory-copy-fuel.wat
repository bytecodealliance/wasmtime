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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108864 "VMStoreContext+0x0"
;;     region3 = 603979776 "VMMemoryDefinition+0x0"
;;     region4 = 603979784 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:12 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001e                               v6 = load.i64 notrap aligned region2 v5
;; @001e                               v7 = iconst.i64 1
;; @001e                               v8 = iadd v6, v7  ; v7 = 1
;; @001e                               v9 = iconst.i64 0
;; @001e                               v10 = icmp sge v8, v9  ; v9 = 0
;; @001e                               brif v10, block2, block3(v8)
;;
;;                                 block2:
;;                                     v61 = iadd.i64 v6, v7  ; v7 = 1
;; @001e                               store notrap aligned region2 v61, v5
;; @001e                               v12 = call fn0(v0)
;; @001e                               v14 = load.i64 notrap aligned region2 v5
;; @001e                               jump block3(v14)
;;
;;                                 block3(v21: i64):
;; @0025                               v22 = iconst.i64 4
;; @0025                               v23 = iadd v21, v22  ; v22 = 4
;; @0025                               v18 = uextend.i64 v4
;; @0025                               v24 = iadd v23, v18
;;                                     v62 = iconst.i64 0
;;                                     v63 = icmp sge v24, v62  ; v62 = 0
;; @0025                               brif v63, block4, block5(v24)
;;
;;                                 block4:
;; @0025                               store.i64 notrap aligned region2 v24, v5
;; @0025                               v28 = call fn0(v0)
;; @0025                               v30 = load.i64 notrap aligned region2 v5
;; @0025                               jump block5(v30)
;;
;;                                 block5(v57: i64):
;; @0025                               v31 = load.i64 notrap aligned region4 v0+64
;; @0025                               v32 = uextend.i64 v2
;; @0025                               v36 = iadd v32, v18
;; @0025                               v37 = icmp ugt v36, v31
;; @0025                               trapnz v37, heap_oob
;; @0025                               v44 = uextend.i64 v3
;; @0025                               v48 = iadd v44, v18
;; @0025                               v49 = icmp ugt v48, v31
;; @0025                               trapnz v49, heap_oob
;; @0025                               v38 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0025                               v42 = iadd v38, v32
;; @0025                               v54 = iadd v38, v44
;; @0025                               call fn1(v0, v42, v54, v18)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               store.i64 notrap aligned region2 v57, v5
;; @0029                               return
;; }
