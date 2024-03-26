;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.trunc)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x6d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movss   %xmm0, 4(%rsp)
;;   32: movss   4(%rsp), %xmm15
;;   39: subq    $4, %rsp
;;   3d: movss   %xmm15, (%rsp)
;;   43: subq    $4, %rsp
;;   47: movss   4(%rsp), %xmm0
;;   4d: movabsq $0, %r11
;;   57: callq   *%r11
;;   5a: addq    $4, %rsp
;;   5e: addq    $4, %rsp
;;   62: movq    0x10(%rsp), %r14
;;   67: addq    $0x18, %rsp
;;   6b: popq    %rbp
;;   6c: retq
;;   6d: ud2
