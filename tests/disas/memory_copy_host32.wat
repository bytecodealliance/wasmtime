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
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+44
;;     gv2 = load.i32 notrap aligned can_move gv0+40
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:4 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i32, v4: i32):
;; @0042                               v7 = load.i32 notrap aligned v0+44
;; @0042                               v8 = uextend.i64 v2
;; @0042                               v9 = uextend.i64 v4
;; @0042                               v10 = iadd v8, v9
;; @0042                               v11 = uextend.i64 v7
;; @0042                               v12 = icmp ule v10, v11
;; @0042                               trapz v12, heap_oob
;; @0042                               v13 = load.i32 notrap aligned can_move v0+40
;; @0042                               v17 = uextend.i64 v3
;; @0042                               v19 = iadd v17, v9
;; @0042                               v21 = icmp ule v19, v11
;; @0042                               trapz v21, heap_oob
;; @0042                               v14 = iadd v13, v2
;; @0042                               v23 = iadd v13, v3
;; @0042                               call fn0(v0, v14, v23, v4)
;; @0046                               jump block1
;;
;;                                 block1:
;; @0046                               return
;; }
;;
;; function u0:1(i32 vmctx, i32, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+44
;;     gv2 = load.i32 notrap aligned can_move gv0+40
;;     gv3 = load.i32 notrap aligned gv0+52
;;     gv4 = load.i32 notrap aligned can_move gv0+48
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:4 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i32, v4: i32):
;; @004f                               v7 = load.i32 notrap aligned v0+52
;; @004f                               v8 = uextend.i64 v2
;; @004f                               v9 = uextend.i64 v4
;; @004f                               v10 = iadd v8, v9
;; @004f                               v11 = uextend.i64 v7
;; @004f                               v12 = icmp ule v10, v11
;; @004f                               trapz v12, heap_oob
;; @004f                               v13 = load.i32 notrap aligned can_move v0+48
;; @004f                               v16 = load.i32 notrap aligned v0+44
;; @004f                               v17 = uextend.i64 v3
;; @004f                               v19 = iadd v17, v9
;; @004f                               v20 = uextend.i64 v16
;; @004f                               v21 = icmp ule v19, v20
;; @004f                               trapz v21, heap_oob
;; @004f                               v22 = load.i32 notrap aligned can_move v0+40
;; @004f                               v14 = iadd v13, v2
;; @004f                               v23 = iadd v22, v3
;; @004f                               call fn0(v0, v14, v23, v4)
;; @0053                               jump block1
;;
;;                                 block1:
;; @0053                               return
;; }
;;
;; function u0:2(i32 vmctx, i32, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+44
;;     gv2 = load.i32 notrap aligned can_move gv0+40
;;     gv3 = load.i32 notrap aligned gv0+60
;;     gv4 = load.i32 notrap aligned can_move gv0+56
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:4 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64, v3: i32, v4: i32):
;; @005c                               v8 = load.i32 notrap aligned v0+60
;; @005c                               v6 = uextend.i64 v4
;; @005c                               v9 = uadd_overflow_trap v2, v6, heap_oob
;; @005c                               v10 = uextend.i64 v8
;; @005c                               v11 = icmp ule v9, v10
;; @005c                               trapz v11, heap_oob
;; @005c                               v13 = load.i32 notrap aligned can_move v0+56
;; @005c                               v16 = load.i32 notrap aligned v0+44
;; @005c                               v17 = uextend.i64 v3
;; @005c                               v19 = iadd v17, v6
;; @005c                               v20 = uextend.i64 v16
;; @005c                               v21 = icmp ule v19, v20
;; @005c                               trapz v21, heap_oob
;; @005c                               v22 = load.i32 notrap aligned can_move v0+40
;; @005c                               v12 = ireduce.i32 v2
;; @005c                               v14 = iadd v13, v12
;; @005c                               v23 = iadd v22, v3
;; @005c                               call fn0(v0, v14, v23, v4)
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
;;
;; function u0:3(i32 vmctx, i32, i64, i64, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+60
;;     gv2 = load.i32 notrap aligned can_move gv0+56
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:4 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64, v3: i64, v4: i64):
;; @0069                               v7 = load.i32 notrap aligned v0+60
;; @0069                               v8 = uadd_overflow_trap v2, v4, heap_oob
;; @0069                               v9 = uextend.i64 v7
;; @0069                               v10 = icmp ule v8, v9
;; @0069                               trapz v10, heap_oob
;; @0069                               v12 = load.i32 notrap aligned can_move v0+56
;; @0069                               v16 = uadd_overflow_trap v3, v4, heap_oob
;; @0069                               v18 = icmp ule v16, v9
;; @0069                               trapz v18, heap_oob
;; @0069                               v11 = ireduce.i32 v2
;; @0069                               v13 = iadd v12, v11
;; @0069                               v19 = ireduce.i32 v3
;; @0069                               v21 = iadd v12, v19
;; @0069                               v22 = ireduce.i32 v4
;; @0069                               call fn0(v0, v13, v21, v22)
;; @006d                               jump block1
;;
;;                                 block1:
;; @006d                               return
;; }
;;
;; function u0:4(i32 vmctx, i32, i64, i64, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+60
;;     gv2 = load.i32 notrap aligned can_move gv0+56
;;     gv3 = load.i32 notrap aligned gv0+68
;;     gv4 = load.i32 notrap aligned can_move gv0+64
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:4 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64, v3: i64, v4: i64):
;; @0076                               v7 = load.i32 notrap aligned v0+68
;; @0076                               v8 = uadd_overflow_trap v2, v4, heap_oob
;; @0076                               v9 = uextend.i64 v7
;; @0076                               v10 = icmp ule v8, v9
;; @0076                               trapz v10, heap_oob
;; @0076                               v12 = load.i32 notrap aligned can_move v0+64
;; @0076                               v15 = load.i32 notrap aligned v0+60
;; @0076                               v16 = uadd_overflow_trap v3, v4, heap_oob
;; @0076                               v17 = uextend.i64 v15
;; @0076                               v18 = icmp ule v16, v17
;; @0076                               trapz v18, heap_oob
;; @0076                               v20 = load.i32 notrap aligned can_move v0+56
;; @0076                               v11 = ireduce.i32 v2
;; @0076                               v13 = iadd v12, v11
;; @0076                               v19 = ireduce.i32 v3
;; @0076                               v21 = iadd v20, v19
;; @0076                               v22 = ireduce.i32 v4
;; @0076                               call fn0(v0, v13, v21, v22)
;; @007a                               jump block1
;;
;;                                 block1:
;; @007a                               return
;; }
;;
;; function u0:5(i32 vmctx, i32, i32, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+60
;;     gv2 = load.i32 notrap aligned can_move gv0+56
;;     gv3 = load.i32 notrap aligned gv0+44
;;     gv4 = load.i32 notrap aligned can_move gv0+40
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:4 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i64, v4: i32):
;; @0083                               v8 = load.i32 notrap aligned v0+44
;; @0083                               v9 = uextend.i64 v2
;; @0083                               v6 = uextend.i64 v4
;; @0083                               v11 = iadd v9, v6
;; @0083                               v12 = uextend.i64 v8
;; @0083                               v13 = icmp ule v11, v12
;; @0083                               trapz v13, heap_oob
;; @0083                               v14 = load.i32 notrap aligned can_move v0+40
;; @0083                               v17 = load.i32 notrap aligned v0+60
;; @0083                               v18 = uadd_overflow_trap v3, v6, heap_oob
;; @0083                               v19 = uextend.i64 v17
;; @0083                               v20 = icmp ule v18, v19
;; @0083                               trapz v20, heap_oob
;; @0083                               v22 = load.i32 notrap aligned can_move v0+56
;; @0083                               v15 = iadd v14, v2
;; @0083                               v21 = ireduce.i32 v3
;; @0083                               v23 = iadd v22, v21
;; @0083                               call fn0(v0, v15, v23, v4)
;; @0087                               jump block1
;;
;;                                 block1:
;; @0087                               return
;; }
