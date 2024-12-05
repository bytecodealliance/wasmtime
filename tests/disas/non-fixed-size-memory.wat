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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0041                               v4 = uextend.i64 v2
;; @0041                               v5 = global_value.i64 gv4
;; @0041                               v6 = icmp uge v4, v5
;; @0041                               trapnz v6, heap_oob
;; @0041                               v7 = global_value.i64 gv5
;; @0041                               v8 = iadd v7, v4
;; @0041                               istore8 little heap v3, v8
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0049                               v4 = uextend.i64 v2
;; @0049                               v5 = global_value.i64 gv4
;; @0049                               v6 = icmp uge v4, v5
;; @0049                               trapnz v6, heap_oob
;; @0049                               v7 = global_value.i64 gv5
;; @0049                               v8 = iadd v7, v4
;; @0049                               v9 = uload8.i32 little heap v8
;; @004c                               jump block1(v9)
;;
;;                                 block1(v3: i32):
;; @004c                               return v3
;; }
