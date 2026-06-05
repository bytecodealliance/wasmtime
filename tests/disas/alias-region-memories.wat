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
;;     region0 = 536870912 "PublicMemory"
;;     region1 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned gv4+8
;;     gv6 = load.i64 notrap aligned readonly can_move gv4
;;     gv7 = load.i64 notrap aligned gv3+88
;;     gv8 = load.i64 notrap aligned readonly can_move gv3+80
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003b                               v18 = load.i64 notrap aligned readonly can_move v0+48
;; @003b                               v6 = load.i64 notrap aligned readonly can_move v18
;; @003b                               v5 = uextend.i64 v2
;; @003b                               v7 = iadd v6, v5
;; @003b                               store little region0 v3, v7
;; @0042                               v9 = load.i64 notrap aligned readonly can_move v0+80
;; @0042                               v10 = iadd v9, v5
;; @0042                               store little region1 v3, v10
;; @004b                               jump block1
;;
;;                                 block1:
;; @004b                               return v3
;; }
