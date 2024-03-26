;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.gt)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x2d(%rip), %xmm0
;;   33: movss   0x2d(%rip), %xmm1
;;   3b: ucomiss %xmm0, %xmm1
;;   3e: movl    $0, %eax
;;   43: seta    %al
;;   47: movl    $0, %r11d
;;   4d: setnp   %r11b
;;   51: andl    %r11d, %eax
;;   54: addq    $0x10, %rsp
;;   58: popq    %rbp
;;   59: retq
;;   5a: ud2
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
;;   60: int     $0xcc
;;   62: orb     $0x40, %al
;;   64: addb    %al, (%rax)
;;   66: addb    %al, (%rax)
;;   68: int     $0xcc
