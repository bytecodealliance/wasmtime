;;! target = "x86_64"
;;! test = "optimize"

(module
  (memory i32 1)

  (func (export "f") (param i32 i32)
    local.get 0
    local.get 1
    i32.store
    local.get 0
    local.get 1
    i32.store
  )
)

;; function u0:0(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 134217752 "VMStoreContext+0x18"
;;     region2 = 1207959552 "VMMemoryDefinition+0x0"
;;     region3 = 1207959560 "VMMemoryDefinition+0x8"
;;     region4 = 402653184 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0029                               v5 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0029                               v4 = uextend.i64 v2
;; @0029                               v6 = iadd v5, v4
;; @0030                               store little region4 v3, v6
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return
;; }
