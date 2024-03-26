;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (param f64) (result i32)
        (local.get 0)
        (local.get 1)
        (f64.ne)
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
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movsd   %xmm0, 8(%rsp)
;;   32: movsd   %xmm1, (%rsp)
;;   37: movsd   (%rsp), %xmm0
;;   3c: movsd   8(%rsp), %xmm1
;;   42: ucomisd %xmm0, %xmm1
;;   46: movl    $0, %eax
;;   4b: setne   %al
;;   4f: movl    $0, %r11d
;;   55: setp    %r11b
;;   59: orq     %r11, %rax
;;   5c: addq    $0x20, %rsp
;;   60: popq    %rbp
;;   61: retq
;;   62: ud2
