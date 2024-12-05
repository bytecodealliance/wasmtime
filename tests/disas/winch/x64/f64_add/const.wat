;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.add)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x1c(%rip), %xmm0
;;       movsd   0x1c(%rip), %xmm1
;;       addsd   %xmm0, %xmm1
;;       movapd  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4a: ud2
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
