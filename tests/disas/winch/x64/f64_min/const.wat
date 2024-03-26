;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.min)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x6c
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x3d(%rip), %xmm0
;;   33: movsd   0x3d(%rip), %xmm1
;;   3b: ucomisd %xmm0, %xmm1
;;   3f: jne     0x5e
;;   45: jp      0x54
;;   4b: orpd    %xmm0, %xmm1
;;   4f: jmp     0x62
;;   54: addsd   %xmm0, %xmm1
;;   58: jp      0x62
;;   5e: minsd   %xmm0, %xmm1
;;   62: movapd  %xmm1, %xmm0
;;   66: addq    $0x10, %rsp
;;   6a: popq    %rbp
;;   6b: retq
;;   6c: ud2
;;   6e: addb    %al, (%rax)
