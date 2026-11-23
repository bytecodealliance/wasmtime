;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ] 

(module
    (func (result v128)
        (f32x4.convert_i32x4_u (v128.const i32x4 0 1 2 3))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x67
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x39(%rip), %xmm0
;;       vpslld  $0x10, %xmm0, %xmm15
;;       vpsrld  $0x10, %xmm15, %xmm15
;;       vpsubd  %xmm15, %xmm0, %xmm0
;;       vcvtdq2ps %xmm15, %xmm15
;;       vpsrld  $1, %xmm0, %xmm0
;;       vcvtdq2ps %xmm0, %xmm0
;;       vaddps  %xmm0, %xmm0, %xmm0
;;       vaddps  %xmm15, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   67: ud2
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rax)
;;   71: addb    %al, (%rax)
;;   73: addb    %al, (%rcx)
;;   75: addb    %al, (%rax)
;;   77: addb    %al, (%rdx)
;;   79: addb    %al, (%rax)
;;   7b: addb    %al, (%rbx)
;;   7d: addb    %al, (%rax)
