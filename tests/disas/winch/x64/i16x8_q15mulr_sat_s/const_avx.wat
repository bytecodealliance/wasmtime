;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i16x8.q15mulr_sat_s (v128.const i16x8 0 1 2 3 4 5 6 7) (v128.const i16x8 7 6 5 4 3 2 1 0))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x57
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x2c(%rip), %xmm0
;;       movdqu  0x34(%rip), %xmm1
;;       vpmulhrsw %xmm0, %xmm1, %xmm1
;;       vpcmpeqw 0x37(%rip), %xmm1, %xmm0
;;       vpxor   %xmm0, %xmm1, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   57: ud2
;;   59: addb    %al, (%rax)
;;   5b: addb    %al, (%rax)
;;   5d: addb    %al, (%rax)
;;   5f: addb    %al, (%rdi)
;;   61: addb    %al, (%rsi)
;;   63: addb    %al, 0x3000400(%rip)
;;   69: addb    %al, (%rdx)
;;   6b: addb    %al, (%rcx)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rax)
;;   71: addb    %al, (%rcx)
;;   73: addb    %al, (%rdx)
;;   75: addb    %al, (%rbx)
;;   77: addb    %al, (%rax, %rax)
;;   7a: addl    $0x7000600, %eax
;;   7f: addb    %al, (%rax)
;;   81: addb    $0x80, (%rax)
;;   84: addb    %al, -0x7fff8000(%rax)
;;   8a: addb    %al, -0x7fff8000(%rax)
