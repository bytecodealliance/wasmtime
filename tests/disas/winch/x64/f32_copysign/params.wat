;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.copysign)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x65
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movss   %xmm0, 4(%rsp)
;;   32: movss   %xmm1, (%rsp)
;;   37: movss   (%rsp), %xmm0
;;   3c: movss   4(%rsp), %xmm1
;;   42: movl    $0x80000000, %r11d
;;   48: movd    %r11d, %xmm15
;;   4d: andps   %xmm15, %xmm0
;;   51: andnps  %xmm1, %xmm15
;;   55: movaps  %xmm15, %xmm1
;;   59: orps    %xmm0, %xmm1
;;   5c: movaps  %xmm1, %xmm0
;;   5f: addq    $0x18, %rsp
;;   63: popq    %rbp
;;   64: retq
;;   65: ud2
