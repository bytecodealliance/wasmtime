;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

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
;;       ja      0x62
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x39(%rip), %xmm0
;;       movdqu  0x41(%rip), %xmm1
;;       vpshufb 0x48(%rip), %xmm1, %xmm1
;;       vpshufb 0x4f(%rip), %xmm0, %xmm15
;;       vpor    %xmm1, %xmm15, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   62: ud2
;;   64: addb    %al, (%rax)
;;   66: addb    %al, (%rax)
;;   68: addb    %al, (%rax)
;;   6a: addb    %al, (%rax)
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: addb    (%rax), %al
;;   72: addb    %al, (%rax)
;;   74: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   78: addl    %eax, (%rax)
;;   7a: addb    %al, (%rax)
;;   7c: addb    %al, (%rax)
;;   7e: addb    %al, (%rax)
;;   80: addl    %eax, (%rax)
;;   82: addb    %al, (%rax)
;;   84: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   88: addb    (%rax), %al
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)
;;   8e: addb    %al, (%rax)
;;   90: addb    %al, (%rcx)
;;   92: addb    (%rbx), %al
;;   94: addb    $6, 0x5048080(%rax)
