;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (memory 1 1)
  (func  (param v128 v128) (result v128)
        (i8x16.min_s
          (local.get 0)
          (local.get 1)
          )
        ))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x52
;;   1c: movq    %rdi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movdqu  %xmm0, 0x10(%rsp)
;;       movdqu  %xmm1, (%rsp)
;;       movdqu  (%rsp), %xmm0
;;       movdqu  0x10(%rsp), %xmm1
;;       vpminsb %xmm1, %xmm0, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x30, %rsp
;;       popq    %rbp
;;       retq
;;   52: ud2
