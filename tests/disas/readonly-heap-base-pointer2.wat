;;! test = "optimize"
;;! target = "x86_64"
;;! flags = ["-Omemory-reservation=0x20000"]

(module
  (memory 1 200 shared)
  (func $load (param i32) (result i32)
    (i32.load (local.get 0)))
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 48 "VMContext+0x30"
;;     region3 = 603979776 "VMMemoryDefinition+0x0"
;;     region4 = 603979784 "VMMemoryDefinition+0x8"
;;     region5 = 201326592 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0022                               v3 = uextend.i64 v2
;; @0022                               v4 = iconst.i64 0x0001_fffc
;; @0022                               v5 = icmp ugt v3, v4  ; v4 = 0x0001_fffc
;; @0022                               v9 = iconst.i64 0
;; @0022                               v6 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0022                               v7 = load.i64 notrap aligned readonly can_move region3 v6
;; @0022                               v8 = iadd v7, v3
;; @0022                               v10 = select_spectre_guard v5, v9, v8  ; v9 = 0
;; @0022                               v11 = load.i32 little region5 v10
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v11
;; }
