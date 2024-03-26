;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo i32)
        (local $bar i32)

        (i32.const 2)
        (local.set $foo)
        (i32.const 3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i32.ge_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    $2, %eax
;;   39: movl    %eax, 4(%rsp)
;;   3d: movl    $3, %eax
;;   42: movl    %eax, (%rsp)
;;   45: movl    (%rsp), %eax
;;   48: movl    4(%rsp), %ecx
;;   4c: cmpl    %eax, %ecx
;;   4e: movl    $0, %ecx
;;   53: setge   %cl
;;   57: movl    %ecx, %eax
;;   59: addq    $0x18, %rsp
;;   5d: popq    %rbp
;;   5e: retq
;;   5f: ud2
