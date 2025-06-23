;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (f64x2.min (v128.const i64x2 1 0) (v128.const i64x2 0 1))
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
;;       vminpd  %xmm0, %xmm1, %xmm15
;;       vminpd  %xmm1, %xmm0, %xmm1
;;       vorpd   %xmm15, %xmm1, %xmm1
;;       vcmpunordpd %xmm1, %xmm0, %xmm0
;;       vorpd   %xmm1, %xmm0, %xmm1
;;       vpsrlq  $0xd, %xmm0, %xmm0
;;       vandnpd %xmm1, %xmm0, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   6b: ud2
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rax)
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
;;   87: addb    %al, (%rax)
;;   89: addb    %al, (%rax)
;;   8b: addb    %al, (%rax)
;;   8d: addb    %al, (%rax)
