;;! target = "x86_64"

;; Test basic code generation for i32 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i32.load8_s") (param i32) (result i32)
    local.get 0
    i32.load8_s))

;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0031                               v3 = uextend.i64 v2
;; @0031                               v4 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0031                               v5 = iadd v4, v3
;; @0031                               v6 = sload8.i32 little region4 v5
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return v6
;; }
