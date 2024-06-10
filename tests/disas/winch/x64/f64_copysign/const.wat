;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const -1.1)
        (f64.const 2.2)
        (f64.copysign)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x67
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x3d(%rip), %xmm0
;;       movsd   0x3d(%rip), %xmm1
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       andpd   %xmm15, %xmm0
;;       andnpd  %xmm1, %xmm15
;;       movapd  %xmm15, %xmm1
;;       orpd    %xmm0, %xmm1
;;       movapd  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   67: ud2
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %bl, -0x66666667(%rdx)
;;   75: cltd
;;   76: addl    %eax, -0x66(%rax)
;;   79: cltd
;;   7a: cltd
;;   7b: cltd
;;   7c: cltd
;;   7d: cltd
;;   7e: int1
