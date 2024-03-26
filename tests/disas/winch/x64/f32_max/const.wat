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
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x69
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x3d(%rip), %xmm0
;;   33: movss   0x3d(%rip), %xmm1
;;   3b: ucomiss %xmm0, %xmm1
;;   3e: jne     0x5c
;;   44: jp      0x52
;;   4a: andps   %xmm0, %xmm1
;;   4d: jmp     0x60
;;   52: addss   %xmm0, %xmm1
;;   56: jp      0x60
;;   5c: maxss   %xmm0, %xmm1
;;   60: movaps  %xmm1, %xmm0
;;   63: addq    $0x10, %rsp
;;   67: popq    %rbp
;;   68: retq
;;   69: ud2
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %cl, %ch
;;   71: int3
;;   72: orb     $0x40, %al
;;   74: addb    %al, (%rax)
;;   76: addb    %al, (%rax)
;;   78: int     $0xcc
