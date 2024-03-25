;;! target = "x86_64"
;;! test = "compile"
;;! flags = "-Ccranelift-sse42 -Ccranelift-has-avx"

(module
  (func (export "f32.pmin") (param f32 f32) (result f32)
    (select
      (local.get 1) (local.get 0)
      (f32.lt (local.get 1) (local.get 0))))
  (func (export "f32.pmax") (param f32 f32) (result f32)
    (select
      (local.get 1) (local.get 0)
      (f32.lt (local.get 0) (local.get 1))))

  (func (export "f64.pmin") (param f64 f64) (result f64)
    (select
      (local.get 1) (local.get 0)
      (f64.lt (local.get 1) (local.get 0))))
  (func (export "f64.pmax") (param f64 f64) (result f64)
    (select
      (local.get 1) (local.get 0)
      (f64.lt (local.get 0) (local.get 1))))

  (func (export "f32x4.pmin") (param v128 v128) (result v128)
    (f32x4.pmin (local.get 0) (local.get 1)))
  (func (export "f32x4.pmax") (param v128 v128) (result v128)
    (f32x4.pmax (local.get 0) (local.get 1)))

  (func (export "f64x2.pmin") (param v128 v128) (result v128)
    (f64x2.pmin (local.get 0) (local.get 1)))
  (func (export "f64x2.pmax") (param v128 v128) (result v128)
    (f64x2.pmax (local.get 0) (local.get 1)))
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: vminss  %xmm0, %xmm1, %xmm0
;;    8: movq    %rbp, %rsp
;;    b: popq    %rbp
;;    c: retq
;;
;; wasm[0]::function[1]:
;;   10: pushq   %rbp
;;   11: movq    %rsp, %rbp
;;   14: vmaxss  %xmm0, %xmm1, %xmm0
;;   18: movq    %rbp, %rsp
;;   1b: popq    %rbp
;;   1c: retq
;;
;; wasm[0]::function[2]:
;;   20: pushq   %rbp
;;   21: movq    %rsp, %rbp
;;   24: vminsd  %xmm0, %xmm1, %xmm0
;;   28: movq    %rbp, %rsp
;;   2b: popq    %rbp
;;   2c: retq
;;
;; wasm[0]::function[3]:
;;   30: pushq   %rbp
;;   31: movq    %rsp, %rbp
;;   34: vmaxsd  %xmm0, %xmm1, %xmm0
;;   38: movq    %rbp, %rsp
;;   3b: popq    %rbp
;;   3c: retq
;;
;; wasm[0]::function[4]:
;;   40: pushq   %rbp
;;   41: movq    %rsp, %rbp
;;   44: vminps  %xmm0, %xmm1, %xmm0
;;   48: movq    %rbp, %rsp
;;   4b: popq    %rbp
;;   4c: retq
;;
;; wasm[0]::function[5]:
;;   50: pushq   %rbp
;;   51: movq    %rsp, %rbp
;;   54: vmaxps  %xmm0, %xmm1, %xmm0
;;   58: movq    %rbp, %rsp
;;   5b: popq    %rbp
;;   5c: retq
;;
;; wasm[0]::function[6]:
;;   60: pushq   %rbp
;;   61: movq    %rsp, %rbp
;;   64: vminpd  %xmm0, %xmm1, %xmm0
;;   68: movq    %rbp, %rsp
;;   6b: popq    %rbp
;;   6c: retq
;;
;; wasm[0]::function[7]:
;;   70: pushq   %rbp
;;   71: movq    %rsp, %rbp
;;   74: vmaxpd  %xmm0, %xmm1, %xmm0
;;   78: movq    %rbp, %rsp
;;   7b: popq    %rbp
;;   7c: retq
