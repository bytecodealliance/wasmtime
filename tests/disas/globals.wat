;;! target = "x86_64"

(module
  (global $x (mut i32) (i32.const 4))
  (memory 1)
  (func $main (local i32)
    (i32.store (i32.const 0) (global.get $x))
  )
  (start $main)
)

;; function u0:0(i64 vmctx, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 469762048 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(0))"
;;     region3 = 603979776 "VMMemoryDefinition+0x0"
;;     region4 = 603979784 "VMMemoryDefinition+0x8"
;;     region5 = 201326592 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0027                               v2 = iconst.i32 0
;; @0029                               v3 = iconst.i32 0
;; @002b                               v4 = load.i32 notrap aligned region2 v0+80
;; @002d                               v5 = uextend.i64 v3  ; v3 = 0
;; @002d                               v6 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @002d                               v7 = iadd v6, v5
;; @002d                               store little region5 v4, v7
;; @0030                               jump block1
;;
;;                                 block1:
;; @0030                               return
;; }
