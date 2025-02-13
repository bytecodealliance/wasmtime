;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (param v128 v128) (result v128)
    (local.get 0)
    (local.get 1)
    (i32x4.extmul_high_i16x8_u)
    ))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x66
;;   1c: movq    %rdi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movdqu  %xmm0, 0x10(%rsp)
;;       movdqu  %xmm1, (%rsp)
;;       movdqu  (%rsp), %xmm0
;;       movdqu  0x10(%rsp), %xmm1
;;       vpxor   %xmm15, %xmm15, %xmm15
;;       vpunpckhwd %xmm15, %xmm0, %xmm0
;;       vpxor   %xmm15, %xmm15, %xmm15
;;       vpunpckhwd %xmm15, %xmm1, %xmm1
;;       vpmulld %xmm0, %xmm1, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x30, %rsp
;;       popq    %rbp
;;       retq
;;   66: ud2
