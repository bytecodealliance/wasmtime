;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.reinterpret_f64)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x3e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0xd(%rip), %xmm0
;;   33: movq    %xmm0, %rax
;;   38: addq    $0x10, %rsp
;;   3c: popq    %rbp
;;   3d: retq
;;   3e: ud2
;;   40: addb    %al, (%rax)
;;   42: addb    %al, (%rax)
;;   44: addb    %al, (%rax)
