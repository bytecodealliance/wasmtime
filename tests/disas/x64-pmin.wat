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
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vminss  %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmaxss  %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vminsd  %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmaxsd  %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vminps  %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmaxps  %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[6]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vminpd  %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmaxpd  %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
