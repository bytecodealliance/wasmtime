;;! target = "x86_64"

(module
  (type (;0;) (func (param i32 i32) (result f64)))
  (func $main (type 0) (param i32 i32) (result f64)
    f64.const 1.0
    local.get 0
    local.get 1
    if (param i32)  ;; label = @2
      i64.load16_s align=1
      drop
    else
      unreachable
    end)
  (table (;0;) 63 255 funcref)
  (memory (;0;) 13 16)
  (export "t1" (table 0))
  (export "m1" (memory 0))
  (export "main" (func $main))
  (export "memory" (memory 0)))

;; function u0:0(i64 vmctx, i64, i32, i32) -> f64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned readonly checked gv3+80
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0049                               v5 = f64const 0x1.0000000000000p0
;; @0056                               brif v3, block2, block4
;;
;;                                 block2:
;; @0058                               v7 = uextend.i64 v2
;; @0058                               v8 = load.i64 notrap aligned readonly checked v0+80
;; @0058                               v9 = iadd v8, v7
;; @0058                               v10 = sload16.i64 little heap v9
;; @005c                               jump block3
;;
;;                                 block4:
;; @005d                               trap user11
;;
;;                                 block3:
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return v5  ; v5 = 0x1.0000000000000p0
;; }
