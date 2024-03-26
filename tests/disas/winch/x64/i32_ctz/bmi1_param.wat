;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_bmi1"]

(module
    (func (param i32) (result i32)
        (local.get 0)
        (i32.ctz)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x3e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %eax
;;   34: tzcntl  %eax, %eax
;;   38: addq    $0x18, %rsp
;;   3c: popq    %rbp
;;   3d: retq
;;   3e: ud2
