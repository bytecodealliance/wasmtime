;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (param v128) (result v128)
        (local.get 0)
        (i32x4.extadd_pairwise_i16x8_s)
        ))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x45
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movdqu  %xmm0, (%rsp)
;;       movdqu  (%rsp), %xmm0
;;       vpmaddwd 0x11(%rip), %xmm0, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   45: ud2
;;   47: addb    %al, (%rax)
;;   49: addb    %al, (%rax)
;;   4b: addb    %al, (%rax)
;;   4d: addb    %al, (%rax)
;;   4f: addb    %al, (%rcx)
;;   51: addb    %al, (%rcx)
;;   53: addb    %al, (%rcx)
;;   55: addb    %al, (%rcx)
;;   57: addb    %al, (%rcx)
;;   59: addb    %al, (%rcx)
;;   5b: addb    %al, (%rcx)
;;   5d: addb    %al, (%rcx)
