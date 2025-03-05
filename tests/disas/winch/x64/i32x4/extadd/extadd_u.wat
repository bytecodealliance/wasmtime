;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (func (param v128) (result v128)
        (local.get 0)
        (i32x4.extadd_pairwise_i16x8_u)
        ))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x55
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movdqu  %xmm0, (%rsp)
;;       movdqu  (%rsp), %xmm0
;;       vpxor   0x21(%rip), %xmm0, %xmm0
;;       vpmaddwd 0x29(%rip), %xmm0, %xmm0
;;       vpaddd  0x31(%rip), %xmm0, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   55: ud2
;;   57: addb    %al, (%rax)
;;   59: addb    %al, (%rax)
;;   5b: addb    %al, (%rax)
;;   5d: addb    %al, (%rax)
;;   5f: addb    %al, (%rax)
;;   61: addb    $0x80, (%rax)
;;   64: addb    %al, -0x7fff8000(%rax)
;;   6a: addb    %al, -0x7fff8000(%rax)
;;   70: addl    %eax, (%rax)
;;   72: addl    %eax, (%rax)
;;   74: addl    %eax, (%rax)
;;   76: addl    %eax, (%rax)
;;   78: addl    %eax, (%rax)
;;   7a: addl    %eax, (%rax)
;;   7c: addl    %eax, (%rax)
;;   7e: addl    %eax, (%rax)
;;   80: addb    %al, (%rax)
;;   82: addl    %eax, (%rax)
;;   84: addb    %al, (%rax)
;;   86: addl    %eax, (%rax)
;;   88: addb    %al, (%rax)
;;   8a: addl    %eax, (%rax)
;;   8c: addb    %al, (%rax)
;;   8e: addl    %eax, (%rax)
