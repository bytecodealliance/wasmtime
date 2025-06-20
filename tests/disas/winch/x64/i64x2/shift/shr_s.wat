;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i64x2.shr_s (v128.const i64x2 1 2) (i32.const 3))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x69
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $3, %eax
;;       movdqu  0x34(%rip), %xmm0
;;       andl    $0x3f, %eax
;;       vmovd   %eax, %xmm15
;;       vmovdqu 0x32(%rip), %xmm1
;;       vpsrlq  %xmm15, %xmm1, %xmm1
;;       vpsrlq  %xmm15, %xmm0, %xmm0
;;       vpxor   %xmm1, %xmm0, %xmm0
;;       vpsubq  %xmm1, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   69: ud2
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rcx)
;;   71: addb    %al, (%rax)
;;   73: addb    %al, (%rax)
;;   75: addb    %al, (%rax)
;;   77: addb    %al, (%rdx)
;;   79: addb    %al, (%rax)
;;   7b: addb    %al, (%rax)
;;   7d: addb    %al, (%rax)
;;   7f: addb    %al, (%rax)
;;   81: addb    %al, (%rax)
;;   83: addb    %al, (%rax)
;;   85: addb    %al, (%rax)
;;   87: addb    $0, (%rax)
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)
