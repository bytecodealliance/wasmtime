;;! target = "x86_64"
;;!
;;! settings = ["enable_heap_access_spectre_mitigation=false"]
;;!
;;! compile = false
;;!
;;! [globals.vmctx]
;;! type = "i64"
;;! vmctx = true
;;!
;;! [globals.heap_base]
;;! type = "i64"
;;! load = { base = "vmctx", offset = 0, readonly = true }
;;!
;;! [globals.heap_bound]
;;! type = "i64"
;;! load = { base = "vmctx", offset = 8, readonly = true }
;;!
;;! [[heaps]]
;;! base = "heap_base"
;;! min_size = 0x10000
;;! max_size = 0x10000
;;! offset_guard_size = 0
;;! index_type = "i32"
;;! style = { kind = "dynamic", bound = "heap_bound" }

;; Test that dynamic memories with `min_size == max_size` don't actually load
;; their dynamic memory bound, since it is a constant.

(module
  (memory 1 1)

  (func (export "do_store") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8 offset=0)

  (func (export "do_load") (param i32) (result i32)
    local.get 0
    i32.load8_u offset=0))

;; function u0:0(i32, i32, i64 vmctx) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i32, v2: i64):
;; @0041                               v3 = uextend.i64 v0
;; @0041                               v4 = iconst.i64 0x0001_0000
;; @0041                               v5 = icmp uge v3, v4  ; v4 = 0x0001_0000
;; @0041                               trapnz v5, heap_oob
;; @0041                               v6 = global_value.i64 gv2
;; @0041                               v7 = iadd v6, v3
;; @0041                               istore8 little heap v1, v7
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return
;; }
;;
;; function u0:1(i32, i64 vmctx) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0049                               v3 = uextend.i64 v0
;; @0049                               v4 = iconst.i64 0x0001_0000
;; @0049                               v5 = icmp uge v3, v4  ; v4 = 0x0001_0000
;; @0049                               trapnz v5, heap_oob
;; @0049                               v6 = global_value.i64 gv2
;; @0049                               v7 = iadd v6, v3
;; @0049                               v8 = uload8.i32 little heap v7
;; @004c                               jump block1(v8)
;;
;;                                 block1(v2: i32):
;; @004c                               return v2
;; }
