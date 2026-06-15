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
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 48 "VMContext+0x30"
;;     region3 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0022                               v4 = uextend.i64 v2
;; @0022                               v5 = iconst.i64 0x0001_fffc
;; @0022                               v6 = icmp ugt v4, v5  ; v5 = 0x0001_fffc
;; @0022                               v10 = iconst.i64 0
;; @0022                               v7 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0022                               v8 = load.i64 notrap aligned readonly can_move v7
;; @0022                               v9 = iadd v8, v4
;; @0022                               v11 = select_spectre_guard v6, v10, v9  ; v10 = 0
;; @0022                               v12 = load.i32 little region3 v11
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v12
;; }
