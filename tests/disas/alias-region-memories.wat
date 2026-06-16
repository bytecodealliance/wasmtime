;;! target = "x86_64"
;;! test = "optimize"

(module
  (import "env" "mem" (memory $imported 1))
  (memory $defined 1)

  (func (export "test") (param i32 i32) (result i32)
    ;; Store to imported memory
    (i32.store $imported (local.get 0) (local.get 1))
    ;; Store to defined memory
    (i32.store $defined (local.get 0) (local.get 1))
    ;; Load from imported memory (should alias with first store)
    (i32.load $imported (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 48 "VMContext+0x30"
;;     region2 = 536870912 "PublicMemory"
;;     region3 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003b                               v6 = load.i64 notrap aligned readonly can_move region1 v0+48
;; @003b                               v7 = load.i64 notrap aligned readonly can_move v6
;; @003b                               v5 = uextend.i64 v2
;; @003b                               v8 = iadd v7, v5
;; @003b                               store little region2 v3, v8
;; @0042                               v10 = load.i64 notrap aligned readonly can_move v0+80
;; @0042                               v11 = iadd v10, v5
;; @0042                               store little region3 v3, v11
;; @004b                               jump block1
;;
;;                                 block1:
;; @004b                               return v3
;; }
