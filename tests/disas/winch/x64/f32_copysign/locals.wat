;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const -1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.copysign
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7d
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movss   0x44(%rip), %xmm0
;;       movss   %xmm0, 4(%rsp)
;;       movss   0x3e(%rip), %xmm0
;;       movss   %xmm0, (%rsp)
;;       movss   (%rsp), %xmm0
;;       movss   4(%rsp), %xmm1
;;       movl    $0x80000000, %r11d
;;       movd    %r11d, %xmm15
;;       andps   %xmm15, %xmm0
;;       andnps  %xmm1, %xmm15
;;       movaps  %xmm15, %xmm1
;;       orps    %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   7d: ud2
;;   7f: addb    %cl, %ch
;;   81: int3
