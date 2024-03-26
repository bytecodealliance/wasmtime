;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i32) (result i32)
        (local.get 0)
        (i32.eqz)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x46
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %eax
;;   34: cmpl    $0, %eax
;;   37: movl    $0, %eax
;;   3c: sete    %al
;;   40: addq    $0x18, %rsp
;;   44: popq    %rbp
;;   45: retq
;;   46: ud2
