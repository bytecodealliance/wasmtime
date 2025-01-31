;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i8x16.swizzle (v128.const i64x2 1 2) (v128.const i64x2 2 1))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x53
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x2c(%rip), %xmm0
;;       movdqu  0x34(%rip), %xmm1
;;       vpaddusb 0x3c(%rip), %xmm0, %xmm0
;;       vpshufb %xmm0, %xmm1, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   53: ud2
;;   55: addb    %al, (%rax)
;;   57: addb    %al, (%rax)
;;   59: addb    %al, (%rax)
;;   5b: addb    %al, (%rax)
;;   5d: addb    %al, (%rax)
;;   5f: addb    %al, (%rdx)
;;   61: addb    %al, (%rax)
;;   63: addb    %al, (%rax)
;;   65: addb    %al, (%rax)
;;   67: addb    %al, (%rcx)
;;   69: addb    %al, (%rax)
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
;;   7f: addb    %dh, 0x70(%rax)
;;   82: jo      0xf4
;;   84: jo      0xf6
;;   86: jo      0xf8
;;   88: jo      0xfa
;;   8a: jo      0xfc
;;   8c: jo      0xfe
;;   8e: jo      0x100
