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
        f32.mul
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x67
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movss   0x34(%rip), %xmm0
;;   3c: movss   %xmm0, 4(%rsp)
;;   42: movss   0x2e(%rip), %xmm0
;;   4a: movss   %xmm0, (%rsp)
;;   4f: movss   (%rsp), %xmm0
;;   54: movss   4(%rsp), %xmm1
;;   5a: mulss   %xmm0, %xmm1
;;   5e: movaps  %xmm1, %xmm0
;;   61: addq    $0x18, %rsp
;;   65: popq    %rbp
;;   66: retq
;;   67: ud2
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %cl, %ch
;;   71: int3
