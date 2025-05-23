;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.div)
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
;;       movss   0x21(%rip), %xmm0
;;       movss   0x21(%rip), %xmm1
;;       divss   %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4f: ud2
;;   51: addb    %al, (%rax)
;;   53: addb    %al, (%rax)
;;   55: addb    %al, (%rax)
;;   57: addb    %cl, %ch
;;   59: int3
;;   5a: orb     $0x40, %al
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
;;   60: int     $0xcc
