;;! target = 'x86_64'
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
;; function u0:0(i64 vmctx, i64, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+80
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:4 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0042                               v7 = load.i64 notrap aligned v0+88
;; @0042                               v8 = uextend.i64 v2
;; @0042                               v9 = uextend.i64 v4
;; @0042                               v10 = iadd v8, v9
;; @0042                               v11 = icmp ule v10, v7
;; @0042                               trapz v11, heap_oob
;; @0042                               v17 = uextend.i64 v3
;; @0042                               v19 = iadd v17, v9
;; @0042                               v20 = icmp ule v19, v7
;; @0042                               trapz v20, heap_oob
;; @0042                               v13 = load.i64 notrap aligned readonly can_move v0+80
;; @0042                               v14 = iadd v13, v8
;; @0042                               v23 = iadd v13, v17
;; @0042                               call fn0(v0, v14, v23, v9)
;; @0046                               jump block1
;;
;;                                 block1:
;; @0046                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+80
;;     gv6 = load.i64 notrap aligned gv3+104
;;     gv7 = load.i64 notrap aligned readonly can_move gv3+96
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:4 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @004f                               v7 = load.i64 notrap aligned v0+104
;; @004f                               v8 = uextend.i64 v2
;; @004f                               v9 = uextend.i64 v4
;; @004f                               v10 = iadd v8, v9
;; @004f                               v11 = icmp ule v10, v7
;; @004f                               trapz v11, heap_oob
;; @004f                               v16 = load.i64 notrap aligned v0+88
;; @004f                               v17 = uextend.i64 v3
;; @004f                               v19 = iadd v17, v9
;; @004f                               v20 = icmp ule v19, v16
;; @004f                               trapz v20, heap_oob
;; @004f                               v13 = load.i64 notrap aligned readonly can_move v0+96
;; @004f                               v14 = iadd v13, v8
;; @004f                               v22 = load.i64 notrap aligned readonly can_move v0+80
;; @004f                               v23 = iadd v22, v17
;; @004f                               call fn0(v0, v14, v23, v9)
;; @0053                               jump block1
;;
;;                                 block1:
;; @0053                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+80
;;     gv6 = load.i64 notrap aligned gv3+120
;;     gv7 = load.i64 notrap aligned can_move gv3+112
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:4 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32, v4: i32):
;; @005c                               v8 = load.i64 notrap aligned v0+120
;; @005c                               v6 = uextend.i64 v4
;; @005c                               v9 = uadd_overflow_trap v2, v6, heap_oob
;; @005c                               v10 = icmp ule v9, v8
;; @005c                               trapz v10, heap_oob
;; @005c                               v11 = load.i64 notrap aligned can_move v0+112
;; @005c                               v14 = load.i64 notrap aligned v0+88
;; @005c                               v15 = uextend.i64 v3
;; @005c                               v17 = iadd v15, v6
;; @005c                               v18 = icmp ule v17, v14
;; @005c                               trapz v18, heap_oob
;; @005c                               v12 = iadd v11, v2
;; @005c                               v20 = load.i64 notrap aligned readonly can_move v0+80
;; @005c                               v21 = iadd v20, v15
;; @005c                               call fn0(v0, v12, v21, v6)
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
;;
;; function u0:3(i64 vmctx, i64, i64, i64, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+120
;;     gv5 = load.i64 notrap aligned can_move gv3+112
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:4 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0069                               v7 = load.i64 notrap aligned v0+120
;; @0069                               v8 = uadd_overflow_trap v2, v4, heap_oob
;; @0069                               v9 = icmp ule v8, v7
;; @0069                               trapz v9, heap_oob
;; @0069                               v10 = load.i64 notrap aligned can_move v0+112
;; @0069                               v14 = uadd_overflow_trap v3, v4, heap_oob
;; @0069                               v15 = icmp ule v14, v7
;; @0069                               trapz v15, heap_oob
;; @0069                               v11 = iadd v10, v2
;; @0069                               v17 = iadd v10, v3
;; @0069                               call fn0(v0, v11, v17, v4)
;; @006d                               jump block1
;;
;;                                 block1:
;; @006d                               return
;; }
;;
;; function u0:4(i64 vmctx, i64, i64, i64, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+120
;;     gv5 = load.i64 notrap aligned can_move gv3+112
;;     gv6 = load.i64 notrap aligned gv3+136
;;     gv7 = load.i64 notrap aligned can_move gv3+128
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:4 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0076                               v7 = load.i64 notrap aligned v0+136
;; @0076                               v8 = uadd_overflow_trap v2, v4, heap_oob
;; @0076                               v9 = icmp ule v8, v7
;; @0076                               trapz v9, heap_oob
;; @0076                               v10 = load.i64 notrap aligned can_move v0+128
;; @0076                               v13 = load.i64 notrap aligned v0+120
;; @0076                               v14 = uadd_overflow_trap v3, v4, heap_oob
;; @0076                               v15 = icmp ule v14, v13
;; @0076                               trapz v15, heap_oob
;; @0076                               v16 = load.i64 notrap aligned can_move v0+112
;; @0076                               v11 = iadd v10, v2
;; @0076                               v17 = iadd v16, v3
;; @0076                               call fn0(v0, v11, v17, v4)
;; @007a                               jump block1
;;
;;                                 block1:
;; @007a                               return
;; }
;;
;; function u0:5(i64 vmctx, i64, i32, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+120
;;     gv5 = load.i64 notrap aligned can_move gv3+112
;;     gv6 = load.i64 notrap aligned gv3+88
;;     gv7 = load.i64 notrap aligned readonly can_move gv3+80
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:4 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64, v4: i32):
;; @0083                               v8 = load.i64 notrap aligned v0+88
;; @0083                               v9 = uextend.i64 v2
;; @0083                               v6 = uextend.i64 v4
;; @0083                               v11 = iadd v9, v6
;; @0083                               v12 = icmp ule v11, v8
;; @0083                               trapz v12, heap_oob
;; @0083                               v17 = load.i64 notrap aligned v0+120
;; @0083                               v18 = uadd_overflow_trap v3, v6, heap_oob
;; @0083                               v19 = icmp ule v18, v17
;; @0083                               trapz v19, heap_oob
;; @0083                               v20 = load.i64 notrap aligned can_move v0+112
;; @0083                               v14 = load.i64 notrap aligned readonly can_move v0+80
;; @0083                               v15 = iadd v14, v9
;; @0083                               v21 = iadd v20, v3
;; @0083                               call fn0(v0, v15, v21, v6)
;; @0087                               jump block1
;;
;;                                 block1:
;; @0087                               return
;; }
