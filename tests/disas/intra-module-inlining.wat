;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[0]--function"
;;! flags = "-C inlining=y -C cranelift-wasmtime_inlining_intra_module=y"

(module
  (func (result i32)
    (i32.const 42))
  (func (result i32)
    (call 0)))

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001b                               jump block1
;;
;;                                 block1:
;; @0019                               v3 = iconst.i32 42
;; @001b                               return v3  ; v3 = 42
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+8
;;     gv5 = load.i64 notrap aligned gv4+24
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001e                               jump block2
;;
;;                                 block2:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block4
;;
;;                                 block4:
;; @0020                               jump block1
;;
;;                                 block1:
;;                                     v6 = iconst.i32 42
;; @0020                               return v6  ; v6 = 42
;; }
