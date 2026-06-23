;;! test = "optimize"
;;! target = "x86_64"
;;! flags = ["-Omemory-reservation=0x20000", "-Omemory-may-move=n"]

(module
  (memory 1)
  (func $load (param i32) (result i32)
    (i32.load (local.get 0)))
)
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
;; @0020                               v3 = uextend.i64 v2
;; @0020                               v4 = iconst.i64 0x0001_fffc
;; @0020                               v5 = icmp ugt v3, v4  ; v4 = 0x0001_fffc
;; @0020                               v8 = iconst.i64 0
;; @0020                               v6 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0020                               v7 = iadd v6, v3
;; @0020                               v9 = select_spectre_guard v5, v8, v7  ; v8 = 0
;; @0020                               v10 = load.i32 little region4 v9
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v10
;; }
