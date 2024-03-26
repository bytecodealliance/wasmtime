;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.ne
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x79
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movss   0x44(%rip), %xmm0
;;   3c: movss   %xmm0, 4(%rsp)
;;   42: movss   0x3e(%rip), %xmm0
;;   4a: movss   %xmm0, (%rsp)
;;   4f: movss   (%rsp), %xmm0
;;   54: movss   4(%rsp), %xmm1
;;   5a: ucomiss %xmm0, %xmm1
;;   5d: movl    $0, %eax
;;   62: setne   %al
;;   66: movl    $0, %r11d
;;   6c: setp    %r11b
;;   70: orl     %r11d, %eax
;;   73: addq    $0x18, %rsp
;;   77: popq    %rbp
;;   78: retq
;;   79: ud2
;;   7b: addb    %al, (%rax)
;;   7d: addb    %al, (%rax)
;;   7f: addb    %cl, %ch
;;   81: int3
