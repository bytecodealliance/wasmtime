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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108864 "VMStoreContext+0x0"
;;     region3 = 603979776 "VMMemoryDefinition+0x0"
;;     region4 = 603979784 "VMMemoryDefinition+0x8"
;;     region5 = 201326592 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:12 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0023                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0023                               v5 = load.i64 notrap aligned region2 v4
;; @0023                               v6 = iconst.i64 1
;; @0023                               v7 = iadd v5, v6  ; v6 = 1
;; @0023                               v8 = iconst.i64 0
;; @0023                               v9 = icmp sge v7, v8  ; v8 = 0
;; @0023                               brif v9, block2, block3(v7)
;;
;;                                 block2:
;;                                     v58 = iadd.i64 v5, v6  ; v6 = 1
;; @0023                               store notrap aligned region2 v58, v4
;; @0023                               v11 = call fn0(v0)
;; @0023                               v13 = load.i64 notrap aligned region2 v4
;; @0023                               jump block3(v13)
;;
;;                                 block3(v43: i64):
;; @002a                               v17 = load.i64 notrap aligned region4 v0+64
;; @002a                               v18 = uextend.i64 v2
;;                                     v47 = iconst.i64 16
;; @002a                               v22 = iadd v18, v47  ; v47 = 16
;; @002a                               v23 = icmp ugt v22, v17
;; @002a                               trapnz v23, heap_oob
;; @002a                               v30 = uextend.i64 v3
;; @002a                               v34 = iadd v30, v47  ; v47 = 16
;; @002a                               v35 = icmp ugt v34, v17
;; @002a                               trapnz v35, heap_oob
;; @002a                               v24 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @002a                               v40 = iadd v24, v30
;; @002a                               v42 = load.i8x16 notrap aligned little region5 v40
;; @002a                               v28 = iadd v24, v18
;; @002a                               store notrap aligned little region5 v42, v28
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               v44 = iconst.i64 20
;; @002e                               v45 = iadd.i64 v43, v44  ; v44 = 20
;; @002e                               store notrap aligned region2 v45, v4
;; @002e                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108864 "VMStoreContext+0x0"
;;     region3 = 603979776 "VMMemoryDefinition+0x0"
;;     region4 = 603979784 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:12 sig0
;;     fn1 = colocated u805306368:2 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0030                               v3 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0030                               v4 = load.i64 notrap aligned region2 v3
;; @0030                               v5 = iconst.i64 1
;; @0030                               v6 = iadd v4, v5  ; v5 = 1
;; @0030                               v7 = iconst.i64 0
;; @0030                               v8 = icmp sge v6, v7  ; v7 = 0
;; @0030                               brif v8, block2, block3(v6)
;;
;;                                 block2:
;;                                     v43 = iadd.i64 v4, v5  ; v5 = 1
;; @0030                               store notrap aligned region2 v43, v3
;; @0030                               v10 = call fn0(v0)
;; @0030                               v12 = load.i64 notrap aligned region2 v3
;; @0030                               jump block3(v12)
;;
;;                                 block3(v29: i64):
;; @0038                               v16 = load.i64 notrap aligned region4 v0+64
;; @0038                               v17 = uextend.i64 v2
;;                                     v33 = iconst.i64 128
;; @0038                               v21 = iadd v17, v33  ; v33 = 128
;; @0038                               v22 = icmp ugt v21, v16
;; @0038                               trapnz v22, heap_oob
;; @0038                               v23 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0038                               v27 = iadd v23, v17
;; @0033                               v14 = iconst.i32 0
;; @0038                               call fn1(v0, v27, v14, v33)  ; v14 = 0, v33 = 128
;; @003b                               jump block1
;;
;;                                 block1:
;; @003b                               v30 = iconst.i64 132
;; @003b                               v31 = iadd.i64 v29, v30  ; v30 = 132
;; @003b                               store notrap aligned region2 v31, v3
;; @003b                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108864 "VMStoreContext+0x0"
;;     region3 = 603979776 "VMMemoryDefinition+0x0"
;;     region4 = 603979784 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:12 sig0
;;     fn1 = colocated u805306368:2 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003d                               v3 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003d                               v4 = load.i64 notrap aligned region2 v3
;; @003d                               v5 = iconst.i64 1
;; @003d                               v6 = iadd v4, v5  ; v5 = 1
;; @003d                               v7 = iconst.i64 0
;; @003d                               v8 = icmp sge v6, v7  ; v7 = 0
;; @003d                               brif v8, block2, block3(v6)
;;
;;                                 block2:
;;                                     v52 = iadd.i64 v4, v5  ; v5 = 1
;; @003d                               store notrap aligned region2 v52, v3
;; @003d                               v10 = call fn0(v0)
;; @003d                               v12 = load.i64 notrap aligned region2 v3
;; @003d                               jump block3(v12)
;;
;;                                 block3(v16: i64):
;; @0045                               v17 = iconst.i64 4
;; @0045                               v18 = iadd v16, v17  ; v17 = 4
;;                                     v53 = iconst.i64 0
;;                                     v54 = icmp sge v18, v53  ; v53 = 0
;; @0045                               brif v54, block4, block5(v18)
;;
;;                                 block4:
;;                                     v55 = iadd.i64 v16, v17  ; v17 = 4
;; @0045                               store notrap aligned region2 v55, v3
;; @0045                               v22 = call fn0(v0)
;; @0045                               v24 = load.i64 notrap aligned region2 v3
;; @0045                               jump block5(v24)
;;
;;                                 block5(v38: i64):
;; @0045                               v25 = load.i64 notrap aligned region4 v0+64
;; @0045                               v26 = uextend.i64 v2
;;                                     v42 = iconst.i64 4096
;; @0045                               v30 = iadd v26, v42  ; v42 = 4096
;; @0045                               v31 = icmp ugt v30, v25
;; @0045                               trapnz v31, heap_oob
;; @0045                               v32 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0045                               v36 = iadd v32, v26
;; @0040                               v14 = iconst.i32 0
;; @0045                               call fn1(v0, v36, v14, v42)  ; v14 = 0, v42 = 4096
;; @0048                               jump block1
;;
;;                                 block1:
;;                                     v56 = iconst.i64 4096
;;                                     v57 = iadd.i64 v38, v56  ; v56 = 4096
;; @0048                               store notrap aligned region2 v57, v3
;; @0048                               return
;; }
