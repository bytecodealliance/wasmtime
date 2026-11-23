;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (param v128) (result v128)
        (local.get 0)
        (i16x8.extadd_pairwise_i8x16_u)
        ))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4c
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movdqu  %xmm0, (%rsp)
;;       movdqu  (%rsp), %xmm0
;;       vpmaddubsw 0xd(%rip), %xmm0, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   4c: ud2
;;   4e: addb    %al, (%rax)
;;   50: addl    %eax, (%rcx)
;;   52: addl    %eax, (%rcx)
;;   54: addl    %eax, (%rcx)
;;   56: addl    %eax, (%rcx)
;;   58: addl    %eax, (%rcx)
;;   5a: addl    %eax, (%rcx)
;;   5c: addl    %eax, (%rcx)
;;   5e: addl    %eax, (%rcx)
