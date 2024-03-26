;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.min
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x88
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movss   0x54(%rip), %xmm0
;;   3c: movss   %xmm0, 4(%rsp)
;;   42: movss   0x4e(%rip), %xmm0
;;   4a: movss   %xmm0, (%rsp)
;;   4f: movss   (%rsp), %xmm0
;;   54: movss   4(%rsp), %xmm1
;;   5a: ucomiss %xmm0, %xmm1
;;   5d: jne     0x7b
;;   63: jp      0x71
;;   69: orps    %xmm0, %xmm1
;;   6c: jmp     0x7f
;;   71: addss   %xmm0, %xmm1
;;   75: jp      0x7f
;;   7b: minss   %xmm0, %xmm1
;;   7f: movaps  %xmm1, %xmm0
;;   82: addq    $0x18, %rsp
;;   86: popq    %rbp
;;   87: retq
;;   88: ud2
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)
;;   8e: addb    %al, (%rax)
;;   90: int     $0xcc
