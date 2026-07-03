;;! target = "x86_64"

(module
  (memory 1)
  (func (export "f32.load") (param i32) (result f32)
    local.get 0
    f32.load))

;; function u0:0(i64 vmctx, i64, i32) -> f32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002e                               v3 = uextend.i64 v2
;; @002e                               v4 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @002e                               v5 = iadd v4, v3
;; @002e                               v6 = load.f32 little region4 v5
;; @0031                               jump block1
;;
;;                                 block1:
;; @0031                               return v6
;; }
