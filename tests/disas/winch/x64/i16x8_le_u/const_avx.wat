;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i16x8.le_u (v128.const i16x8 7 6 5 4 3 2 1 0) (v128.const i16x8 0 1 2 3 4 5 6 7))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x2c(%rip), %xmm0
;;       movdqu  0x34(%rip), %xmm1
;;       vpminuw %xmm0, %xmm1, %xmm0
;;       vpcmpeqw %xmm0, %xmm1, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4f: ud2
;;   51: addb    %al, (%rax)
;;   53: addb    %al, (%rax)
;;   55: addb    %al, (%rax)
;;   57: addb    %al, (%rax)
;;   59: addb    %al, (%rax)
;;   5b: addb    %al, (%rax)
;;   5d: addb    %al, (%rax)
;;   5f: addb    %al, (%rax)
;;   61: addb    %al, (%rcx)
;;   63: addb    %al, (%rdx)
;;   65: addb    %al, (%rbx)
;;   67: addb    %al, (%rax, %rax)
;;   6a: addl    $0x7000600, %eax
;;   6f: addb    %al, (%rdi)
;;   71: addb    %al, (%rsi)
;;   73: addb    %al, 0x3000400(%rip)
;;   79: addb    %al, (%rdx)
;;   7b: addb    %al, (%rcx)
;;   7d: addb    %al, (%rax)
