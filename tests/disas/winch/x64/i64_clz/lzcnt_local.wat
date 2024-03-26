;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_lzcnt"]

(module
    (func (result i64)
        (local $foo i64)

        (i64.const 2)
        (local.set $foo)

        (local.get $foo)
        (i64.clz)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movq    $2, %rax
;;   3b: movq    %rax, (%rsp)
;;   3f: movq    (%rsp), %rax
;;   43: lzcntq  %rax, %rax
;;   48: addq    $0x18, %rsp
;;   4c: popq    %rbp
;;   4d: retq
;;   4e: ud2
