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
;; function u0:0(i64 vmctx, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 96 "VMContext+0x60"
;;     region2 = 80 "VMContext+0x50"
;;     region3 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+64
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0028                               v3 = iconst.i32 0
;; @0030                               v6 = load.i64 notrap aligned readonly can_move region2 v0+80
;; @0030                               v5 = load.i64 notrap aligned readonly can_move region1 v0+96
;; @0039                               v12 = iconst.i64 0x0001_0000
;; @0039                               v16 = iconst.i64 0
;; @0039                               v14 = load.i64 notrap aligned readonly can_move v0+56
;; @003e                               v18 = iconst.i32 1
;; @002e                               jump block2(v3)  ; v3 = 0
;;
;;                                 block2(v8: i32):
;; @0030                               call_indirect.i64 sig0, v6(v5, v0)
;;                                     v21 = iconst.i32 0
;; @0036                               v9 = iadd.i32 v2, v8
;; @0039                               v11 = uextend.i64 v9
;;                                     v22 = iconst.i64 0x0001_0000
;;                                     v23 = icmp ugt v11, v22  ; v22 = 0x0001_0000
;; @0039                               v15 = iadd.i64 v14, v11
;;                                     v24 = iconst.i64 0
;;                                     v25 = select_spectre_guard v23, v24, v15  ; v24 = 0
;; @0039                               store little region3 v21, v25  ; v21 = 0
;;                                     v26 = iconst.i32 1
;;                                     v27 = iadd v8, v26  ; v26 = 1
;; @0043                               jump block2(v27)
;; }
