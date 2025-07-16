;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O memory-may-move=y -O memory-reservation=0"

(module
  (import "" "" (func $imp))

  ;; Minimum and maximum sizes are equal. Therefore, this memory should never
  ;; move, regardless of any other memory-related settings.
  (memory 1 1)

  ;; And therefore, the heap base should be marked read-only and can-move and
  ;; should get code-motioned up out of the loop below, even though we call some
  ;; foreign function in the loop body.
  (func $f (param $base i32)
    (local $i i32)

    (local.set $i (i32.const 0))

    (loop
      ;; Call a foreign function. This would usually otherwise defeat
      ;; code-motioning the heap base out of the loop.
      (call $imp)

      ;; Do a memory operation, which should use the heap base, but should be
      ;; code-motioned above the loop, because its load should be marked both
      ;; read-only and can-move.
      (i32.store (i32.add (local.get $base) (local.get $i)) (i32.const 0))

      ;; Increment `i` and continue the loop.
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br 0)
    )
  )
)
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+64
;;     gv5 = load.i64 notrap aligned readonly can_move checked gv3+56
;;     sig0 = (i64 vmctx, i64) tail
;;     fn0 = u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0028                               v3 = iconst.i32 0
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v0+72
;; @0030                               v6 = load.i64 notrap aligned readonly can_move v0+88
;; @0039                               v13 = iconst.i64 0x0001_0000
;; @0039                               v17 = iconst.i64 0
;; @0039                               v15 = load.i64 notrap aligned readonly can_move checked v0+56
;; @003e                               v19 = iconst.i32 1
;; @002e                               jump block2(v3)  ; v3 = 0
;;
;;                                 block2(v9: i32):
;; @0030                               call_indirect.i64 sig0, v7(v6, v0)
;;                                     v22 = iconst.i32 0
;; @0036                               v10 = iadd.i32 v2, v9
;; @0039                               v12 = uextend.i64 v10
;;                                     v23 = iconst.i64 0x0001_0000
;;                                     v24 = icmp ugt v12, v23  ; v23 = 0x0001_0000
;; @0039                               v16 = iadd.i64 v15, v12
;;                                     v25 = iconst.i64 0
;;                                     v26 = select_spectre_guard v24, v25, v16  ; v25 = 0
;; @0039                               store little heap v22, v26  ; v22 = 0
;;                                     v27 = iconst.i32 1
;;                                     v28 = iadd v9, v27  ; v27 = 1
;; @0043                               jump block2(v28)
;; }
