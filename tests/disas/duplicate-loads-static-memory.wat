;;! target = "x86_64"
;;! test = "optimize"

(module
  (memory (export "memory") 1)
  (func (export "load-without-offset") (param i32) (result i32 i32)
    local.get 0
    i32.load
    local.get 0
    i32.load
  )
  (func (export "load-with-offset") (param i32) (result i32 i32)
    local.get 0
    i32.load offset=1234
    local.get 0
    i32.load offset=1234
  )
)

;; function u0:0(i64 vmctx, i64, i32) -> i32, i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     region4 = 536870912 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0057                               v4 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0057                               v3 = uextend.i64 v2
;; @0057                               v5 = iadd v4, v3
;; @0057                               v6 = load.i32 little region4 v5
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return v6, v6
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32, i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     region4 = 536870912 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0064                               v4 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0064                               v3 = uextend.i64 v2
;; @0064                               v5 = iadd v4, v3
;; @0064                               v6 = iconst.i64 1234
;; @0064                               v7 = iadd v5, v6  ; v6 = 1234
;; @0064                               v8 = load.i32 little region4 v7
;; @006e                               jump block1
;;
;;                                 block1:
;; @006e                               return v8, v8
;; }
