;;! target = 'pulley32'
;;! test = 'optimize'
;;! flags = '-Wcustom-page-sizes'

(module
  (memory $m32 1)
  (memory $m64 i64 1)

  (memory $m32p1 65536 (pagesize 1))
  (memory $m64p1 i64 65536 (pagesize 1))

  (func $fill32 (param i32 i32)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m32
  )

  (func $fill64 (param i64 i64)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m64
  )

  (func $fill32p1 (param i32 i32)
    local.get 0
    i32.const 0
    local.get 1
    memory.fill $m32p1
  )

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
;; function u0:0(i32 vmctx, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+48
;;     gv2 = load.i32 notrap aligned can_move gv0+44
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:5 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i32):
;; @003c                               v6 = load.i32 notrap aligned v0+48
;; @003c                               v7 = uextend.i64 v2
;; @003c                               v8 = uextend.i64 v3
;; @003c                               v9 = iadd v7, v8
;; @003c                               v10 = uextend.i64 v6
;; @003c                               v11 = icmp ule v9, v10
;; @003c                               trapz v11, heap_oob
;; @003c                               v13 = load.i32 notrap aligned can_move v0+44
;; @003c                               v14 = iadd v13, v2
;; @0038                               v4 = iconst.i32 0
;; @003c                               call fn0(v0, v14, v4, v3)  ; v4 = 0
;; @003f                               jump block1
;;
;;                                 block1:
;; @003f                               return
;; }
;;
;; function u0:1(i32 vmctx, i32, i64, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+56
;;     gv2 = load.i32 notrap aligned can_move gv0+52
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:5 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64, v3: i64):
;; @0048                               v6 = load.i32 notrap aligned v0+56
;; @0048                               v7 = uadd_overflow_trap v2, v3, heap_oob
;; @0048                               v8 = uextend.i64 v6
;; @0048                               v9 = icmp ule v7, v8
;; @0048                               trapz v9, heap_oob
;; @0048                               v12 = load.i32 notrap aligned can_move v0+52
;; @0048                               v11 = ireduce.i32 v2
;; @0048                               v13 = iadd v12, v11
;; @0044                               v4 = iconst.i32 0
;; @0048                               v14 = ireduce.i32 v3
;; @0048                               call fn0(v0, v13, v4, v14)  ; v4 = 0
;; @004b                               jump block1
;;
;;                                 block1:
;; @004b                               return
;; }
;;
;; function u0:2(i32 vmctx, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+64
;;     gv2 = load.i32 notrap aligned can_move gv0+60
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:5 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i32):
;; @0054                               v6 = load.i32 notrap aligned v0+64
;; @0054                               v7 = uextend.i64 v2
;; @0054                               v8 = uextend.i64 v3
;; @0054                               v9 = iadd v7, v8
;; @0054                               v10 = uextend.i64 v6
;; @0054                               v11 = icmp ule v9, v10
;; @0054                               trapz v11, heap_oob
;; @0054                               v13 = load.i32 notrap aligned can_move v0+60
;; @0054                               v14 = iadd v13, v2
;; @0050                               v4 = iconst.i32 0
;; @0054                               call fn0(v0, v14, v4, v3)  ; v4 = 0
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return
;; }
;;
;; function u0:3(i32 vmctx, i32, i64, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+72
;;     gv2 = load.i32 notrap aligned can_move gv0+68
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:5 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64, v3: i64):
;; @0060                               v6 = load.i32 notrap aligned v0+72
;; @0060                               v7 = uadd_overflow_trap v2, v3, heap_oob
;; @0060                               v8 = uextend.i64 v6
;; @0060                               v9 = icmp ule v7, v8
;; @0060                               trapz v9, heap_oob
;; @0060                               v12 = load.i32 notrap aligned can_move v0+68
;; @0060                               v11 = ireduce.i32 v2
;; @0060                               v13 = iadd v12, v11
;; @005c                               v4 = iconst.i32 0
;; @0060                               v14 = ireduce.i32 v3
;; @0060                               call fn0(v0, v13, v4, v14)  ; v4 = 0
;; @0063                               jump block1
;;
;;                                 block1:
;; @0063                               return
;; }
;;
;; function u0:4(i32 vmctx, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i32 notrap aligned gv0+80
;;     gv2 = load.i32 notrap aligned readonly can_move gv0+76
;;     sig0 = (i32 vmctx, i32, i32, i32) tail
;;     fn0 = colocated u805306368:5 sig0
;;
;;                                 block0(v0: i32, v1: i32, v2: i32, v3: i32):
;; @006c                               v6 = load.i32 notrap aligned v0+80
;; @006c                               v7 = uextend.i64 v2
;; @006c                               v8 = uextend.i64 v3
;; @006c                               v9 = iadd v7, v8
;; @006c                               v10 = uextend.i64 v6
;; @006c                               v11 = icmp ule v9, v10
;; @006c                               trapz v11, heap_oob
;; @006c                               v13 = load.i32 notrap aligned readonly can_move v0+76
;; @006c                               v14 = iadd v13, v2
;; @0068                               v4 = iconst.i32 0
;; @006c                               call fn0(v0, v14, v4, v3)  ; v4 = 0
;; @006f                               jump block1
;;
;;                                 block1:
;; @006f                               return
;; }
