;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i32x4.le_s (v128.const i32x4 3 2 1 0) (v128.const i32x4 0 1 2 3))
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
;;       vpminsd %xmm0, %xmm1, %xmm0
;;       vpcmpeqd %xmm0, %xmm1, %xmm1
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
;;   61: addb    %al, (%rax)
;;   63: addb    %al, (%rcx)
;;   65: addb    %al, (%rax)
;;   67: addb    %al, (%rdx)
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rbx)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rbx)
;;   71: addb    %al, (%rax)
;;   73: addb    %al, (%rdx)
;;   75: addb    %al, (%rax)
;;   77: addb    %al, (%rcx)
;;   79: addb    %al, (%rax)
;;   7b: addb    %al, (%rax)
;;   7d: addb    %al, (%rax)
