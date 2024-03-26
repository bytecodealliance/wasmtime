;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.mul)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movss   %xmm0, 4(%rsp)
;;   32: movss   %xmm1, (%rsp)
;;   37: movss   (%rsp), %xmm0
;;   3c: movss   4(%rsp), %xmm1
;;   42: mulss   %xmm0, %xmm1
;;   46: movaps  %xmm1, %xmm0
;;   49: addq    $0x18, %rsp
;;   4d: popq    %rbp
;;   4e: retq
;;   4f: ud2
