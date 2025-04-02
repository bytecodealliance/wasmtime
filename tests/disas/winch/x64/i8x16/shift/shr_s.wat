;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i8x16.shr_s (v128.const i64x2 1 2) (i32.const 3))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $3, %eax
;;       movdqu  0x37(%rip), %xmm0
;;       andl    $7, %eax
;;       addl    $8, %eax
;;       vmovd   %eax, %xmm15
;;       vpunpcklbw %xmm0, %xmm0, %xmm1
;;       vpunpckhbw %xmm0, %xmm0, %xmm2
;;       vpsraw  %xmm15, %xmm1, %xmm1
;;       vpsraw  %xmm15, %xmm2, %xmm2
;;       vpacksswb %xmm2, %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5f: ud2
;;   61: addb    %al, (%rax)
;;   63: addb    %al, (%rax)
;;   65: addb    %al, (%rax)
;;   67: addb    %al, (%rax)
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
