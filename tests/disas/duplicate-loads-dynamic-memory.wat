;;! target = "x86_64"
;;! test = "optimize"
;;! flags = [
;;!   "-Ccranelift-enable-heap-access-spectre-mitigation",
;;!   "-Oopt-level=s",
;;!   "-Ostatic-memory-maximum-size=0",
;;! ]

(module
  (memory (export "memory") 0)
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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 603979776 "VMMemoryDefinition+0x0"
;;     region3 = 603979784 "VMMemoryDefinition+0x8"
;;     region4 = 134217728 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0057                               v4 = load.i64 notrap aligned region3 v0+64
;; @0057                               v6 = load.i64 notrap aligned can_move region2 v0+56
;; @0057                               v3 = uextend.i64 v2
;; @0057                               v5 = icmp ugt v3, v4
;; @0057                               v8 = iconst.i64 0
;; @0057                               v7 = iadd v6, v3
;; @0057                               v9 = select_spectre_guard v5, v8, v7  ; v8 = 0
;; @0057                               v10 = load.i32 little region4 v9
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return v10, v10
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32, i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 603979776 "VMMemoryDefinition+0x0"
;;     region3 = 603979784 "VMMemoryDefinition+0x8"
;;     region4 = 134217728 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0064                               v4 = load.i64 notrap aligned region3 v0+64
;; @0064                               v6 = load.i64 notrap aligned can_move region2 v0+56
;; @0064                               v3 = uextend.i64 v2
;; @0064                               v5 = icmp ugt v3, v4
;; @0064                               v10 = iconst.i64 0
;; @0064                               v7 = iadd v6, v3
;; @0064                               v8 = iconst.i64 1234
;; @0064                               v9 = iadd v7, v8  ; v8 = 1234
;; @0064                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0064                               v12 = load.i32 little region4 v11
;; @006e                               jump block1
;;
;;                                 block1:
;; @006e                               return v12, v12
;; }
