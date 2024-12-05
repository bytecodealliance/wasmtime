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
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x68
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x3c(%rip), %xmm0
;;       movsd   0x3c(%rip), %xmm1
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
;;   68: ud2
;;   6a: addb    %al, (%rax)
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
