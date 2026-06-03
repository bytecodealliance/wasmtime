;;! target = 'x86_64'
;;! test = 'optimize'

;; A constant-length `memory.copy` is expanded inline as wide loads followed by
;; stores (every byte is loaded before any is stored, so overlapping ranges keep
;; `memmove` semantics) instead of calling the `memory_copy` libcall.

(module
  (memory 1)
  (func $copy (param i32 i32)
    (memory.copy (local.get 0) (local.get 1) (i32.const 16))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+64
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+56
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0024                               v6 = load.i64 notrap aligned v0+64
;; @0024                               v7 = uextend.i64 v2
;;                                     v35 = iconst.i64 16
;; @0024                               v11 = iadd v7, v35  ; v35 = 16
;; @0024                               v12 = icmp ugt v11, v6
;; @0024                               trapnz v12, heap_oob
;; @0024                               v20 = uextend.i64 v3
;; @0024                               v24 = iadd v20, v35  ; v35 = 16
;; @0024                               v25 = icmp ugt v24, v6
;; @0024                               trapnz v25, heap_oob
;; @0024                               v13 = load.i64 notrap aligned readonly can_move v0+56
;; @0024                               v30 = iadd v13, v20
;; @0024                               v32 = load.i8x16 notrap aligned little v30
;; @0024                               v17 = iadd v13, v7
;; @0024                               store notrap aligned little v32, v17
;; @0028                               jump block1
;;
;;                                 block1:
;; @0028                               return
;; }
