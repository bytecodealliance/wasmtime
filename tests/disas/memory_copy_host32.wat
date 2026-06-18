;;! target = 'pulley32'
;;! test = 'optimize'

(module
  (memory $m32_a 1)
  (memory $m32_b 1)
  (memory $m64_a i64 1)
  (memory $m64_b i64 1)

  (func $m32_to_same (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m32_a $m32_a
  )

  (func $m32_to_m32 (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m32_b $m32_a
  )

  (func $m32_to_m64 (param i64 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m64_a $m32_a
  )

  (func $m64_to_same (param i64 i64 i64)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m64_a $m64_a
  )

  (func $m64_to_m64 (param i64 i64 i64)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m64_b $m64_a
  )

  (func $m64_to_m32 (param i32 i64 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy $m32_a $m64_a
  )
)
;; function u0:0(i32 vmctx, i32, i32, i32, i32) tail {
;;     region0 = 2415919104 "VMMemoryDefinition+0x0"
;;     region1 = 2415919108 "VMMemoryDefinition+0x4"
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:1 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i32, v4: i32):
;; @0042                               v5 = load.i32 notrap aligned region1 v0+44
;; @0042                               v6 = uextend.i64 v2
;; @0042                               v7 = uextend.i64 v4
;; @0042                               v10 = iadd v6, v7
;; @0042                               v11 = uextend.i64 v5
;; @0042                               v12 = icmp ugt v10, v11
;; @0042                               trapnz v12, heap_oob
;; @0042                               v13 = load.i32 notrap aligned can_move region0 v0+40
;; @0042                               v18 = uextend.i64 v3
;; @0042                               v22 = iadd v18, v7
;; @0042                               v24 = icmp ugt v22, v11
;; @0042                               trapnz v24, heap_oob
;; @0042                               v16 = iadd v13, v2
;; @0042                               v28 = iadd v13, v3
;; @0042                               call fn0(v0, v16, v28, v4)
;; @0046                               jump block1
;;
;;                                 block1:
;; @0046                               return
;; }
;;
;; function u0:1(i32 vmctx, i32, i32, i32, i32) tail {
;;     region0 = 2415919104 "VMMemoryDefinition+0x0"
;;     region1 = 2415919108 "VMMemoryDefinition+0x4"
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:1 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i32, v4: i32):
;; @004f                               v5 = load.i32 notrap aligned region1 v0+52
;; @004f                               v6 = uextend.i64 v2
;; @004f                               v7 = uextend.i64 v4
;; @004f                               v10 = iadd v6, v7
;; @004f                               v11 = uextend.i64 v5
;; @004f                               v12 = icmp ugt v10, v11
;; @004f                               trapnz v12, heap_oob
;; @004f                               v13 = load.i32 notrap aligned can_move region0 v0+48
;; @004f                               v17 = load.i32 notrap aligned region1 v0+44
;; @004f                               v18 = uextend.i64 v3
;; @004f                               v22 = iadd v18, v7
;; @004f                               v23 = uextend.i64 v17
;; @004f                               v24 = icmp ugt v22, v23
;; @004f                               trapnz v24, heap_oob
;; @004f                               v25 = load.i32 notrap aligned can_move region0 v0+40
;; @004f                               v16 = iadd v13, v2
;; @004f                               v28 = iadd v25, v3
;; @004f                               call fn0(v0, v16, v28, v4)
;; @0053                               jump block1
;;
;;                                 block1:
;; @0053                               return
;; }
;;
;; function u0:2(i32 vmctx, i32, i64, i32, i32) tail {
;;     region0 = 2415919104 "VMMemoryDefinition+0x0"
;;     region1 = 2415919108 "VMMemoryDefinition+0x4"
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:1 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64, v3: i32, v4: i32):
;; @005c                               v6 = load.i32 notrap aligned region1 v0+60
;; @005c                               v5 = uextend.i64 v4
;; @005c                               v7 = uadd_overflow_trap v2, v5, heap_oob
;; @005c                               v8 = uextend.i64 v6
;; @005c                               v9 = icmp ugt v7, v8
;; @005c                               trapnz v9, heap_oob
;; @005c                               v10 = load.i32 notrap aligned can_move region0 v0+56
;; @005c                               v15 = load.i32 notrap aligned region1 v0+44
;; @005c                               v16 = uextend.i64 v3
;; @005c                               v20 = iadd v16, v5
;; @005c                               v21 = uextend.i64 v15
;; @005c                               v22 = icmp ugt v20, v21
;; @005c                               trapnz v22, heap_oob
;; @005c                               v23 = load.i32 notrap aligned can_move region0 v0+40
;; @005c                               v11 = ireduce.i32 v2
;; @005c                               v14 = iadd v10, v11
;; @005c                               v26 = iadd v23, v3
;; @005c                               call fn0(v0, v14, v26, v4)
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
;;
;; function u0:3(i32 vmctx, i32, i64, i64, i64) tail {
;;     region0 = 2415919104 "VMMemoryDefinition+0x0"
;;     region1 = 2415919108 "VMMemoryDefinition+0x4"
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:1 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64, v3: i64, v4: i64):
;; @0069                               v5 = load.i32 notrap aligned region1 v0+60
;; @0069                               v6 = uadd_overflow_trap v2, v4, heap_oob
;; @0069                               v7 = uextend.i64 v5
;; @0069                               v8 = icmp ugt v6, v7
;; @0069                               trapnz v8, heap_oob
;; @0069                               v9 = load.i32 notrap aligned can_move region0 v0+56
;; @0069                               v15 = uadd_overflow_trap v3, v4, heap_oob
;; @0069                               v17 = icmp ugt v15, v7
;; @0069                               trapnz v17, heap_oob
;; @0069                               v10 = ireduce.i32 v2
;; @0069                               v13 = iadd v9, v10
;; @0069                               v19 = ireduce.i32 v3
;; @0069                               v22 = iadd v9, v19
;; @0069                               v23 = ireduce.i32 v4
;; @0069                               call fn0(v0, v13, v22, v23)
;; @006d                               jump block1
;;
;;                                 block1:
;; @006d                               return
;; }
;;
;; function u0:4(i32 vmctx, i32, i64, i64, i64) tail {
;;     region0 = 2415919104 "VMMemoryDefinition+0x0"
;;     region1 = 2415919108 "VMMemoryDefinition+0x4"
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:1 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64, v3: i64, v4: i64):
;; @0076                               v5 = load.i32 notrap aligned region1 v0+68
;; @0076                               v6 = uadd_overflow_trap v2, v4, heap_oob
;; @0076                               v7 = uextend.i64 v5
;; @0076                               v8 = icmp ugt v6, v7
;; @0076                               trapnz v8, heap_oob
;; @0076                               v9 = load.i32 notrap aligned can_move region0 v0+64
;; @0076                               v14 = load.i32 notrap aligned region1 v0+60
;; @0076                               v15 = uadd_overflow_trap v3, v4, heap_oob
;; @0076                               v16 = uextend.i64 v14
;; @0076                               v17 = icmp ugt v15, v16
;; @0076                               trapnz v17, heap_oob
;; @0076                               v18 = load.i32 notrap aligned can_move region0 v0+56
;; @0076                               v10 = ireduce.i32 v2
;; @0076                               v13 = iadd v9, v10
;; @0076                               v19 = ireduce.i32 v3
;; @0076                               v22 = iadd v18, v19
;; @0076                               v23 = ireduce.i32 v4
;; @0076                               call fn0(v0, v13, v22, v23)
;; @007a                               jump block1
;;
;;                                 block1:
;; @007a                               return
;; }
;;
;; function u0:5(i32 vmctx, i32, i32, i64, i32) tail {
;;     region0 = 2415919104 "VMMemoryDefinition+0x0"
;;     region1 = 2415919108 "VMMemoryDefinition+0x4"
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:1 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i64, v4: i32):
;; @0083                               v6 = load.i32 notrap aligned region1 v0+44
;; @0083                               v7 = uextend.i64 v2
;; @0083                               v5 = uextend.i64 v4
;; @0083                               v11 = iadd v7, v5
;; @0083                               v12 = uextend.i64 v6
;; @0083                               v13 = icmp ugt v11, v12
;; @0083                               trapnz v13, heap_oob
;; @0083                               v14 = load.i32 notrap aligned can_move region0 v0+40
;; @0083                               v18 = load.i32 notrap aligned region1 v0+60
;; @0083                               v19 = uadd_overflow_trap v3, v5, heap_oob
;; @0083                               v20 = uextend.i64 v18
;; @0083                               v21 = icmp ugt v19, v20
;; @0083                               trapnz v21, heap_oob
;; @0083                               v22 = load.i32 notrap aligned can_move region0 v0+56
;; @0083                               v17 = iadd v14, v2
;; @0083                               v23 = ireduce.i32 v3
;; @0083                               v26 = iadd v22, v23
;; @0083                               call fn0(v0, v17, v26, v4)
;; @0087                               jump block1
;;
;;                                 block1:
;; @0087                               return
;; }
