;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wcustom-page-sizes'

(module
  (memory $m32 1)
  (func $fill32 (param i32 i32)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m32
  )

  (memory $m64 i64 1)
  (func $fill64 (param i64 i64)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m64
  )

  (memory $m32p1 65536 (pagesize 1))
  (func $fill32p1 (param i32 i32)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m32p1
  )

  (memory $m64p1 i64 65536 (pagesize 1))
  (func $fill64p1 (param i64 i64)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m64p1
  )

  (memory $empty 0 0)
  (func $fill-empty (param i32 i32)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $empty
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003c                               v5 = load.i64 notrap aligned region3 v0+96
;; @003c                               v6 = uextend.i64 v2
;; @003c                               v7 = uextend.i64 v3
;; @003c                               v10 = iadd v6, v7
;; @003c                               v11 = icmp ugt v10, v5
;; @003c                               trapnz v11, heap_oob
;; @003c                               v12 = load.i64 notrap aligned readonly can_move region2 v0+88
;; @003c                               v16 = iadd v12, v6
;; @0038                               v4 = iconst.i32 0
;; @003c                               call fn0(v0, v16, v4, v7)  ; v4 = 0
;; @003f                               jump block1
;;
;;                                 block1:
;; @003f                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i64, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64):
;; @0048                               v5 = load.i64 notrap aligned region3 v0+112
;; @0048                               v6 = uadd_overflow_trap v2, v3, heap_oob
;; @0048                               v7 = icmp ugt v6, v5
;; @0048                               trapnz v7, heap_oob
;; @0048                               v8 = load.i64 notrap aligned can_move region2 v0+104
;; @0048                               v11 = iadd v8, v2
;; @0044                               v4 = iconst.i32 0
;; @0048                               call fn0(v0, v11, v4, v3)  ; v4 = 0
;; @004b                               jump block1
;;
;;                                 block1:
;; @004b                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0054                               v5 = load.i64 notrap aligned region3 v0+128
;; @0054                               v6 = uextend.i64 v2
;; @0054                               v7 = uextend.i64 v3
;; @0054                               v10 = iadd v6, v7
;; @0054                               v11 = icmp ugt v10, v5
;; @0054                               trapnz v11, heap_oob
;; @0054                               v12 = load.i64 notrap aligned readonly can_move region2 v0+120
;; @0054                               v16 = iadd v12, v6
;; @0050                               v4 = iconst.i32 0
;; @0054                               call fn0(v0, v16, v4, v7)  ; v4 = 0
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return
;; }
;;
;; function u0:3(i64 vmctx, i64, i64, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64):
;; @0060                               v5 = load.i64 notrap aligned region3 v0+144
;; @0060                               v6 = uadd_overflow_trap v2, v3, heap_oob
;; @0060                               v7 = icmp ugt v6, v5
;; @0060                               trapnz v7, heap_oob
;; @0060                               v8 = load.i64 notrap aligned can_move region2 v0+136
;; @0060                               v11 = iadd v8, v2
;; @005c                               v4 = iconst.i32 0
;; @0060                               call fn0(v0, v11, v4, v3)  ; v4 = 0
;; @0063                               jump block1
;;
;;                                 block1:
;; @0063                               return
;; }
;;
;; function u0:4(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @006c                               v5 = load.i64 notrap aligned region3 v0+160
;; @006c                               v6 = uextend.i64 v2
;; @006c                               v7 = uextend.i64 v3
;; @006c                               v10 = iadd v6, v7
;; @006c                               v11 = icmp ugt v10, v5
;; @006c                               trapnz v11, heap_oob
;; @006c                               v12 = load.i64 notrap aligned readonly can_move region2 v0+152
;; @006c                               v16 = iadd v12, v6
;; @0068                               v4 = iconst.i32 0
;; @006c                               call fn0(v0, v16, v4, v7)  ; v4 = 0
;; @006f                               jump block1
;;
;;                                 block1:
;; @006f                               return
;; }
