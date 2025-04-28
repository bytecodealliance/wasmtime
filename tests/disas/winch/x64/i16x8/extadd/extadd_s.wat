;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (param v128) (result v128)
        (local.get 0)
        (i16x8.extadd_pairwise_i8x16_s)
        ))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x51
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movdqu  %xmm0, (%rsp)
;;       movdqu  (%rsp), %xmm0
;;       movdqu  0x1d(%rip), %xmm15
;;       vpmaddubsw %xmm0, %xmm15, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   51: ud2
;;   53: addb    %al, (%rax)
;;   55: addb    %al, (%rax)
;;   57: addb    %al, (%rax)
;;   59: addb    %al, (%rax)
;;   5b: addb    %al, (%rax)
;;   5d: addb    %al, (%rax)
;;   5f: addb    %al, (%rcx)
;;   61: addl    %eax, (%rcx)
;;   63: addl    %eax, (%rcx)
;;   65: addl    %eax, (%rcx)
;;   67: addl    %eax, (%rcx)
;;   69: addl    %eax, (%rcx)
;;   6b: addl    %eax, (%rcx)
;;   6d: addl    %eax, (%rcx)
