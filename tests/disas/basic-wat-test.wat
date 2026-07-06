;;! target = "x86_64"

(module
  (memory 0)
  (func (param i32 i32) (result i32)
    local.get 0
    i32.load
    local.get 1
    i32.load
    i32.add))

;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 603979776 "VMMemoryDefinition+0x0"
;;     region3 = 603979784 "VMMemoryDefinition+0x8"
;;     region4 = 201326592 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0021                               v4 = uextend.i64 v2
;; @0021                               v5 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0021                               v6 = iadd v5, v4
;; @0021                               v7 = load.i32 little region4 v6
;; @0026                               v8 = uextend.i64 v3
;; @0026                               v9 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0026                               v10 = iadd v9, v8
;; @0026                               v11 = load.i32 little region4 v10
;; @0029                               v12 = iadd v7, v11
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return v12
;; }
