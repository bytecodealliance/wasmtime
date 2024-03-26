;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.min)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x70
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movss   %xmm0, 4(%rsp)
;;   32: movss   %xmm1, (%rsp)
;;   37: movss   (%rsp), %xmm0
;;   3c: movss   4(%rsp), %xmm1
;;   42: ucomiss %xmm0, %xmm1
;;   45: jne     0x63
;;   4b: jp      0x59
;;   51: orps    %xmm0, %xmm1
;;   54: jmp     0x67
;;   59: addss   %xmm0, %xmm1
;;   5d: jp      0x67
;;   63: minss   %xmm0, %xmm1
;;   67: movaps  %xmm1, %xmm0
;;   6a: addq    $0x18, %rsp
;;   6e: popq    %rbp
;;   6f: retq
;;   70: ud2
