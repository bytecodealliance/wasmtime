;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.copysign)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x6e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movsd   %xmm0, 8(%rsp)
;;   32: movsd   %xmm1, (%rsp)
;;   37: movsd   (%rsp), %xmm0
;;   3c: movsd   8(%rsp), %xmm1
;;   42: movabsq $9223372036854775808, %r11
;;   4c: movq    %r11, %xmm15
;;   51: andpd   %xmm15, %xmm0
;;   56: andnpd  %xmm1, %xmm15
;;   5b: movapd  %xmm15, %xmm1
;;   60: orpd    %xmm0, %xmm1
;;   64: movapd  %xmm1, %xmm0
;;   68: addq    $0x20, %rsp
;;   6c: popq    %rbp
;;   6d: retq
;;   6e: ud2
