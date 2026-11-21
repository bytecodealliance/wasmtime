;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const 1.32)
        (f32.sqrt)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x44
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x11(%rip), %xmm0
;;       sqrtss  %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   44: ud2
;;   46: addb    %al, (%rax)
;;   48: retq
;;   49: cmc
;;   4a: testb   $0x3f, %al
