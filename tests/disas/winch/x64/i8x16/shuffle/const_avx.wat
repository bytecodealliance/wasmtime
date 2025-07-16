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
;;       ja      0x63
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x39(%rip), %xmm0
;;       movdqu  0x41(%rip), %xmm1
;;       vpshufb 0x48(%rip), %xmm1, %xmm1
;;       vpshufb 0x4f(%rip), %xmm0, %xmm15
;;       vpor    %xmm15, %xmm1, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   63: ud2
;;   65: addb    %al, (%rax)
;;   67: addb    %al, (%rax)
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rdx)
;;   71: addb    %al, (%rax)
;;   73: addb    %al, (%rax)
;;   75: addb    %al, (%rax)
;;   77: addb    %al, (%rcx)
;;   79: addb    %al, (%rax)
;;   7b: addb    %al, (%rax)
;;   7d: addb    %al, (%rax)
;;   7f: addb    %al, (%rcx)
;;   81: addb    %al, (%rax)
;;   83: addb    %al, (%rax)
;;   85: addb    %al, (%rax)
;;   87: addb    %al, (%rdx)
;;   89: addb    %al, (%rax)
;;   8b: addb    %al, (%rax)
;;   8d: addb    %al, (%rax)
;;   8f: addb    %al, (%rax)
;;   91: addl    %eax, (%rdx)
;;   93: addl    0x4808080(%rax), %eax
;;   99: addl    $0x80800706, %eax
;;   9e: addb    $4, -0x7f7f7f80(%rax)
;;   a5: addl    $0x80800706, %eax
