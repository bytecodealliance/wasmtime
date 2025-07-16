;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (f32x4.min (v128.const f32x4 3 2 1 0) (v128.const f32x4 0 1 2 3))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6b
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x39(%rip), %xmm0
;;       movdqu  0x41(%rip), %xmm1
;;       vminps  %xmm0, %xmm1, %xmm15
;;       vminps  %xmm1, %xmm0, %xmm1
;;       vorps   %xmm15, %xmm1, %xmm1
;;       vcmpunordps %xmm1, %xmm0, %xmm0
;;       vorps   %xmm1, %xmm0, %xmm1
;;       vpsrld  $0xa, %xmm0, %xmm0
;;       vandnps %xmm1, %xmm0, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   6b: ud2
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rax)
;;   71: addb    %al, (%rax)
;;   73: addb    %al, (%rax)
;;   75: addb    %al, 0x3f(%rax)
;;   7b: addb    %al, (%rax)
;;   7e: addb    %al, (%rax)
;;   82: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   89: addb    %al, 0x3f(%rax)
