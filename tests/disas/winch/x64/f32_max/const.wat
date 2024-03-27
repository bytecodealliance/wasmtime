;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.max)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x69
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x3d(%rip), %xmm0
;;       movss   0x3d(%rip), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       jne     0x5c
;;       jp      0x52
;;   4a: andps   %xmm0, %xmm1
;;       jmp     0x60
;;   52: addss   %xmm0, %xmm1
;;       jp      0x60
;;   5c: maxss   %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   69: ud2
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %cl, %ch
;;   71: int3
;;   72: orb     $0x40, %al
;;   74: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   78: int     $0xcc
