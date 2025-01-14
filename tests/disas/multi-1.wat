;;! target = "x86_64"

(module
  (func (export "multiBlock") (param i64 i32) (result i32 i64 f64)
    (local.get 1)
    (local.get 0)
    (block (param i32 i64) (result i32 i64 f64)
      (f64.const 1234.5))))

;; function u0:0(i64 vmctx, i64, i64, i32) -> i32, i64, f64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @003a                               v10 = f64const 0x1.34a0000000000p10
;; @0043                               jump block2
;;
;;                                 block2:
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return v3, v2, v10  ; v10 = 0x1.34a0000000000p10
;; }
