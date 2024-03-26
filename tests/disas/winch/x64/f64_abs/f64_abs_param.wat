;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.abs)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x50
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movsd   %xmm0, (%rsp)
;;   31: movsd   (%rsp), %xmm0
;;   36: movabsq $0x7fffffffffffffff, %r11
;;   40: movq    %r11, %xmm15
;;   45: andpd   %xmm15, %xmm0
;;   4a: addq    $0x18, %rsp
;;   4e: popq    %rbp
;;   4f: retq
;;   50: ud2
