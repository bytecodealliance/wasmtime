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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1207959560 "VMFunctionImport+0x8"
;;     region4 = 603979776 "VMMemoryDefinition+0x0"
;;     region5 = 603979784 "VMMemoryDefinition+0x8"
;;     region6 = 201326592 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0028                               v3 = iconst.i32 0
;; @0030                               v6 = load.i64 notrap aligned readonly can_move region3 v0+80
;; @0030                               v5 = load.i64 notrap aligned readonly can_move region2 v0+96
;; @0039                               v12 = iconst.i64 0x0001_0000
;; @0039                               v16 = iconst.i64 0
;; @0039                               v14 = load.i64 notrap aligned readonly can_move region4 v0+56
;; @003e                               v18 = iconst.i32 1
;; @002e                               jump block2(v3)  ; v3 = 0
;;
;;                                 block2(v8: i32):
;; @0030                               call_indirect.i64 sig0, v6(v5, v0)
;;                                     v20 = iconst.i32 0
;; @0036                               v9 = iadd.i32 v2, v8
;; @0039                               v11 = uextend.i64 v9
;;                                     v21 = iconst.i64 0x0001_0000
;;                                     v22 = icmp ugt v11, v21  ; v21 = 0x0001_0000
;; @0039                               v15 = iadd.i64 v14, v11
;;                                     v23 = iconst.i64 0
;;                                     v24 = select_spectre_guard v22, v23, v15  ; v23 = 0
;; @0039                               store little region6 v20, v24  ; v20 = 0
;;                                     v25 = iconst.i32 1
;;                                     v26 = iadd v8, v25  ; v25 = 1
;; @0043                               jump block2(v26)
;; }
