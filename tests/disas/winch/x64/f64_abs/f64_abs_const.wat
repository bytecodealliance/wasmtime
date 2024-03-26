;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.abs)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x1d(%rip), %xmm0
;;   33: movabsq $0x7fffffffffffffff, %r11
;;   3d: movq    %r11, %xmm15
;;   42: andpd   %xmm15, %xmm0
;;   47: addq    $0x10, %rsp
;;   4b: popq    %rbp
;;   4c: retq
;;   4d: ud2
;;   4f: addb    %bl, (%rdi)
;;   51: testl   %ebp, %ebx
;;   53: pushq   %rcx
