;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local i32)

        (local.get 0)
        (i32.extend16_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x41
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    4(%rsp), %eax
;;   38: movswl  %ax, %eax
;;   3b: addq    $0x18, %rsp
;;   3f: popq    %rbp
;;   40: retq
;;   41: ud2
