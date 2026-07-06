;;! target = "x86_64"
;;! flags = [
;;!   "-Ccranelift-enable-heap-access-spectre-mitigation=false",
;;!   "-Ostatic-memory-maximum-size=0",
;;!   "-Odynamic-memory-guard-size=0",
;;! ]

;; Dual test to `fixed-size-memory.wat` that checks that we _don't_ use a
;; constant for the heap bound when `min_size != max_size`.

(module
  (memory 1 2)

  (func (export "do_store") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8 offset=0)

  (func (export "do_load") (param i32) (result i32)
    local.get 0
    i32.load8_u offset=0))

;; function u0:0(i64 vmctx, i64, i32, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0041                               v4 = uextend.i64 v2
;; @0041                               v5 = load.i64 notrap aligned region3 v0+64
;; @0041                               v6 = icmp uge v4, v5
;; @0041                               trapnz v6, heap_oob
;; @0041                               v7 = load.i64 notrap aligned can_move region2 v0+56
;; @0041                               v8 = iadd v7, v4
;; @0041                               istore8 little region4 v3, v8
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
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
;; @0049                               v3 = uextend.i64 v2
;; @0049                               v4 = load.i64 notrap aligned region3 v0+64
;; @0049                               v5 = icmp uge v3, v4
;; @0049                               trapnz v5, heap_oob
;; @0049                               v6 = load.i64 notrap aligned can_move region2 v0+56
;; @0049                               v7 = iadd v6, v3
;; @0049                               v8 = uload8.i32 little region4 v7
;; @004c                               jump block1
;;
;;                                 block1:
;; @004c                               return v8
;; }
