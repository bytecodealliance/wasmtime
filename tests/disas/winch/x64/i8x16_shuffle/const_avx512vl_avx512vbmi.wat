;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx512vl", "-Ccranelift-has-avx512vbmi" ]

(module
    (func (result v128)
        v128.const i64x2 1 2
        v128.const i64x2 2 1
        i8x16.shuffle 0 1 2 3 20 21 22 23 4 5 6 7 24 25 26 27
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x54
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x2c(%rip), %xmm0
;;       movdqu  0x34(%rip), %xmm1
;;       movdqu  0x3c(%rip), %xmm2
;;       vpermi2b %xmm0, %xmm1, %xmm2
;;       movdqa  %xmm2, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   54: ud2
;;   56: addb    %al, (%rax)
;;   58: addb    %al, (%rax)
;;   5a: addb    %al, (%rax)
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
;;   60: addb    (%rax), %al
;;   62: addb    %al, (%rax)
;;   64: addb    %al, (%rax)
;;   66: addb    %al, (%rax)
;;   68: addl    %eax, (%rax)
;;   6a: addb    %al, (%rax)
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: addl    %eax, (%rax)
;;   72: addb    %al, (%rax)
;;   74: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   78: addb    (%rax), %al
;;   7a: addb    %al, (%rax)
;;   7c: addb    %al, (%rax)
;;   7e: addb    %al, (%rax)
;;   80: addb    %al, (%rcx)
;;   82: addb    (%rbx), %al
;;   84: adcb    $0x15, %al
