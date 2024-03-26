;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.max)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x73
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movsd   %xmm0, 8(%rsp)
;;   32: movsd   %xmm1, (%rsp)
;;   37: movsd   (%rsp), %xmm0
;;   3c: movsd   8(%rsp), %xmm1
;;   42: ucomisd %xmm0, %xmm1
;;   46: jne     0x65
;;   4c: jp      0x5b
;;   52: andpd   %xmm0, %xmm1
;;   56: jmp     0x69
;;   5b: addsd   %xmm0, %xmm1
;;   5f: jp      0x69
;;   65: maxsd   %xmm0, %xmm1
;;   69: movapd  %xmm1, %xmm0
;;   6d: addq    $0x20, %rsp
;;   71: popq    %rbp
;;   72: retq
;;   73: ud2
