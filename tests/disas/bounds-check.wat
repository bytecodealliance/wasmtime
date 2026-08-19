;;! test = "optimize"
;;! target = "x86_64"
;;! flags = ["-Omemory-reservation=0x8000000", "-Omemory-guard-size=0x100000000", "-Omemory-may-move=n"]

(module
  (memory 16)
  (func $store (param i32)
    ;; No offset. But because we have a 4 GiB guard, this needs no bounds check.
    local.get 0
    i32.const 0
    i32.store8 0

    ;; The greatest possible offset that can ever be in bounds. Again, no
    ;; bounds check.
    local.get 0
    i32.const 0
    i32.store8 0 offset=134217727

    ;; The greatest encodable offset. This will never be in bounds, given
    ;; our memory reservation size, so optimization isn't a concern.
    local.get 0
    i32.const 0
    i32.store8 0 offset=4294967295
  )
  (export "store" (func $store))
)
;; function u0:0(i64 vmctx, i64, i32) tail {
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
;;                                     v26 = iconst.i8 0
;; @002c                               v5 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @002c                               v4 = uextend.i64 v2
;; @002c                               v6 = iadd v5, v4
;; @002c                               store little region4 v26, v6  ; v26 = 0
;; @0033                               v12 = iconst.i64 0x07ff_ffff
;; @0033                               v13 = iadd v6, v12  ; v12 = 0x07ff_ffff
;; @0033                               store little region4 v26, v13  ; v26 = 0
;; @003d                               v17 = load.i64 notrap aligned region3 v0+64
;; @003d                               v18 = icmp ugt v4, v17
;; @003d                               v23 = iconst.i64 0
;; @003d                               v21 = iconst.i64 0xffff_ffff
;; @003d                               v22 = iadd v6, v21  ; v21 = 0xffff_ffff
;; @003d                               v24 = select_spectre_guard v18, v23, v22  ; v23 = 0
;; @003d                               store little region4 v26, v24  ; v26 = 0
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return
;; }
