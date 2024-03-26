;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_lzcnt"]

(module
    (func (result i32)
        (local $foo i32)

        (i32.const 2)
        (local.set $foo)

        (local.get $foo)
        (i32.clz)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4b
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    $2, %eax
;;   39: movl    %eax, 4(%rsp)
;;   3d: movl    4(%rsp), %eax
;;   41: lzcntl  %eax, %eax
;;   45: addq    $0x18, %rsp
;;   49: popq    %rbp
;;   4a: retq
;;   4b: ud2
