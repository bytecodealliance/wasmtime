;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        f32.const 1.0
        i32.reinterpret_f32
        drop
        f32.const 1.0
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4c
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x19(%rip), %xmm0
;;       movd    %xmm0, %eax
;;       movss   0xd(%rip), %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4c: ud2
;;   4e: addb    %al, (%rax)
;;   50: addb    %al, (%rax)
