;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
;;! flags = "-C inlining=n -Wconcurrency-support=n"

;; `$M`'s memory is unambiguous despite being exported and used by component
;; model libcall intrinsics when transcoding strings, and `$M` gets the precise
;; `DefinedMemory` alias region for it.

(component
  (core module $M
    (memory (export "mem") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (i32.const 0)
    )
    (func (export "f") (param i32 i32)
      (i32.store (local.get 0) (local.get 1))
    )
  )

  (core instance $m (instantiate $M))

  (func (export "f") (param "s" string)
    (canon lift (core func $m "f")
      (memory $m "mem")
      (realloc (func $m "realloc"))
    )
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @004a                               jump block1
;;
;;                                 block1:
;; @0048                               v6 = iconst.i32 0
;; @004a                               return v6  ; v6 = 0
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 193 "VMMemoryDefinition+0x0"
;;     region3 = 213 "VMMemoryDefinition+0x8"
;;     region4 = 39 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0051                               v5 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0051                               v4 = uextend.i64 v2
;; @0051                               v6 = iadd v5, v4
;; @0051                               store little region4 v3, v6
;; @0054                               jump block1
;;
;;                                 block1:
;; @0054                               return
;; }
