;;! target = 'x86_64'
;;! test = 'compile'
;;! flags = '-Ccranelift-has-sse41 -Ccranelift-has-avx'

(module
  (func $e1 (param v128 v128) (result i32)
    local.get 0
    local.get 1
    i8x16.eq
    i8x16.all_true)

  (func $e2 (param v128 v128) (result i32)
    local.get 0
    local.get 1
    i8x16.ne
    v128.any_true
    i32.eqz)

  (func $band (param v128 v128) (result i32)
    local.get 0
    local.get 1
    v128.and
    v128.any_true)

  (func $band_not (param v128 v128) (result i32)
    local.get 0
    local.get 1
    v128.not
    v128.and
    v128.any_true)
)
;; wasm[0]::function[0]::e1:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpxor   %xmm1, %xmm0, %xmm5
;;       vptest  %xmm5, %xmm5
;;       sete    %r8b
;;       movzbl  %r8b, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::e2:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpxor   %xmm1, %xmm0, %xmm5
;;       vptest  %xmm5, %xmm5
;;       sete    %r8b
;;       movzbl  %r8b, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]::band:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vptest  %xmm1, %xmm0
;;       setne   %sil
;;       movzbl  %sil, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]::band_not:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vptest  %xmm0, %xmm1
;;       setae   %sil
;;       movzbl  %sil, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
