;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.floor)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x62
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movsd   %xmm0, (%rsp)
;;   31: movsd   (%rsp), %xmm15
;;   37: subq    $8, %rsp
;;   3b: movsd   %xmm15, (%rsp)
;;   41: movsd   (%rsp), %xmm0
;;   46: movabsq $0, %r11
;;   50: callq   *%r11
;;   53: addq    $8, %rsp
;;   57: movq    0x10(%rsp), %r14
;;   5c: addq    $0x18, %rsp
;;   60: popq    %rbp
;;   61: retq
;;   62: ud2
