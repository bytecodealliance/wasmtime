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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1275068416 "VMMemoryImport+0x0"
;;     region3 = 603979776 "VMMemoryDefinition+0x0"
;;     region4 = 603979784 "VMMemoryDefinition+0x8"
;;     region5 = 134217728 "PublicMemory"
;;     region6 = 201326592 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003b                               v5 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @003b                               v6 = load.i64 notrap aligned readonly can_move region3 v5
;; @003b                               v4 = uextend.i64 v2
;; @003b                               v7 = iadd v6, v4
;; @003b                               store little region5 v3, v7
;; @0042                               v9 = load.i64 notrap aligned readonly can_move region3 v0+80
;; @0042                               v10 = iadd v9, v4
;; @0042                               store little region6 v3, v10
;; @004b                               jump block1
;;
;;                                 block1:
;; @004b                               return v3
;; }
