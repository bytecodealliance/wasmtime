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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+96
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+88
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003c                               v7 = load.i64 notrap aligned v0+96
;; @003c                               v8 = uextend.i64 v2
;; @003c                               v9 = uextend.i64 v3
;; @003c                               v10 = iadd v8, v9
;; @003c                               v11 = icmp ule v10, v7
;; @003c                               trapz v11, heap_oob
;; @003c                               v13 = load.i64 notrap aligned readonly can_move v0+88
;; @003c                               v14 = iadd v13, v8
;; @0038                               v4 = iconst.i32 0
;; @003c                               call fn0(v0, v14, v4, v9)  ; v4 = 0
;; @003f                               jump block1
;;
;;                                 block1:
;; @003f                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i64, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+112
;;     gv5 = load.i64 notrap aligned can_move gv3+104
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64):
;; @0048                               v7 = load.i64 notrap aligned v0+112
;; @0048                               v8 = uadd_overflow_trap v2, v3, heap_oob
;; @0048                               v9 = icmp ule v8, v7
;; @0048                               trapz v9, heap_oob
;; @0048                               v10 = load.i64 notrap aligned can_move v0+104
;; @0048                               v11 = iadd v10, v2
;; @0044                               v4 = iconst.i32 0
;; @0048                               call fn0(v0, v11, v4, v3)  ; v4 = 0
;; @004b                               jump block1
;;
;;                                 block1:
;; @004b                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+128
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+120
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0054                               v7 = load.i64 notrap aligned v0+128
;; @0054                               v8 = uextend.i64 v2
;; @0054                               v9 = uextend.i64 v3
;; @0054                               v10 = iadd v8, v9
;; @0054                               v11 = icmp ule v10, v7
;; @0054                               trapz v11, heap_oob
;; @0054                               v13 = load.i64 notrap aligned readonly can_move v0+120
;; @0054                               v14 = iadd v13, v8
;; @0050                               v4 = iconst.i32 0
;; @0054                               call fn0(v0, v14, v4, v9)  ; v4 = 0
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return
;; }
;;
;; function u0:3(i64 vmctx, i64, i64, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+144
;;     gv5 = load.i64 notrap aligned can_move gv3+136
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64):
;; @0060                               v7 = load.i64 notrap aligned v0+144
;; @0060                               v8 = uadd_overflow_trap v2, v3, heap_oob
;; @0060                               v9 = icmp ule v8, v7
;; @0060                               trapz v9, heap_oob
;; @0060                               v10 = load.i64 notrap aligned can_move v0+136
;; @0060                               v11 = iadd v10, v2
;; @005c                               v4 = iconst.i32 0
;; @0060                               call fn0(v0, v11, v4, v3)  ; v4 = 0
;; @0063                               jump block1
;;
;;                                 block1:
;; @0063                               return
;; }
;;
;; function u0:4(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+160
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+152
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @006c                               v7 = load.i64 notrap aligned v0+160
;; @006c                               v8 = uextend.i64 v2
;; @006c                               v9 = uextend.i64 v3
;; @006c                               v10 = iadd v8, v9
;; @006c                               v11 = icmp ule v10, v7
;; @006c                               trapz v11, heap_oob
;; @006c                               v13 = load.i64 notrap aligned readonly can_move v0+152
;; @006c                               v14 = iadd v13, v8
;; @0068                               v4 = iconst.i32 0
;; @006c                               call fn0(v0, v14, v4, v9)  ; v4 = 0
;; @006f                               jump block1
;;
;;                                 block1:
;; @006f                               return
;; }
