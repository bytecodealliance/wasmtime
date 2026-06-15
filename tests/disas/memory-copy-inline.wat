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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     region4 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0024                               v5 = load.i64 notrap aligned region3 v0+64
;; @0024                               v6 = uextend.i64 v2
;;                                     v31 = iconst.i64 16
;; @0024                               v10 = iadd v6, v31  ; v31 = 16
;; @0024                               v11 = icmp ugt v10, v5
;; @0024                               trapnz v11, heap_oob
;; @0024                               v18 = uextend.i64 v3
;; @0024                               v22 = iadd v18, v31  ; v31 = 16
;; @0024                               v23 = icmp ugt v22, v5
;; @0024                               trapnz v23, heap_oob
;; @0024                               v12 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0024                               v28 = iadd v12, v18
;; @0024                               v30 = load.i8x16 notrap aligned little region4 v28
;; @0024                               v16 = iadd v12, v6
;; @0024                               store notrap aligned little region4 v30, v16
;; @0028                               jump block1
;;
;;                                 block1:
;; @0028                               return
;; }
