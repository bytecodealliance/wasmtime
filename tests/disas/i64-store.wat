;;! target = "x86_64"

;; Test basic code generation for i32 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i64.store") (param i32 i64)
    local.get 0
    local.get 1
    i64.store))

;; function u0:0(i64 vmctx, i64, i32, i64) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0031                               v4 = uextend.i64 v2
;; @0031                               v5 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0031                               v6 = iadd v5, v4
;; @0031                               store little region4 v3, v6
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
;; }
