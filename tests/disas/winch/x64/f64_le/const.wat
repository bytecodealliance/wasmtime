;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.le)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x1d(%rip), %xmm0
;;   33: movsd   0x1d(%rip), %xmm1
;;   3b: ucomisd %xmm1, %xmm0
;;   3f: movl    $0, %eax
;;   44: setae   %al
;;   48: addq    $0x10, %rsp
;;   4c: popq    %rbp
;;   4d: retq
;;   4e: ud2
