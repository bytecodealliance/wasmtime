;;! target = "x86_64"
;;! test = "winch"
(module
  (func (param f64 f64 f64 f64) (result f32 f64)
    f64.const 0
    local.get 0
    i64.const 0
    f64.const 0
    i64.const 0
    local.get 0
    f64.const 0
    i64.const 1
    i32.const 1
    i64.const 1
    f32.const 0
    local.get 0

    i32.const 0
    br_if 0

    drop
    drop
    drop
    drop
    drop
    drop
    i64.reinterpret_f64
    i64.const 0
    i64.xor
    drop
    drop
    drop
    drop
    drop
    drop
    f32.const 0
    f64.const 0
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rsi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x58, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x130
;;   1c: movq    %rsi, %r14
;;       subq    $0x40, %rsp
;;       movq    %rsi, 0x38(%rsp)
;;       movq    %rdx, 0x30(%rsp)
;;       movsd   %xmm0, 0x28(%rsp)
;;       movsd   %xmm1, 0x20(%rsp)
;;       movsd   %xmm2, 0x18(%rsp)
;;       movsd   %xmm3, 0x10(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movl    $0, %eax
;;       movsd   0x28(%rsp), %xmm15
;;       subq    $8, %rsp
;;       movsd   %xmm15, (%rsp)
;;       movsd   0x30(%rsp), %xmm15
;;       subq    $8, %rsp
;;       movsd   %xmm15, (%rsp)
;;       movsd   0x38(%rsp), %xmm15
;;       subq    $8, %rsp
;;       movsd   %xmm15, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       addq    $8, %rsp
;;       subq    $4, %rsp
;;       movss   0x8e(%rip), %xmm15
;;       movss   %xmm15, (%rsp)
;;       testl   %eax, %eax
;;       je      0xcd
;;   b8: movl    (%rsp), %r11d
;;       movl    %r11d, 0x10(%rsp)
;;       addq    $0x10, %rsp
;;       jmp     0x110
;;   cd: addq    $4, %rsp
;;       movsd   (%rsp), %xmm0
;;       addq    $8, %rsp
;;       movq    %xmm0, %rax
;;       xorq    $0, %rax
;;       addq    $8, %rsp
;;       movsd   0x46(%rip), %xmm0
;;       subq    $4, %rsp
;;       movss   0x2e(%rip), %xmm15
;;       movss   %xmm15, (%rsp)
;;       movq    0xc(%rsp), %rax
;;       movss   (%rsp), %xmm15
;;       addq    $4, %rsp
;;       movss   %xmm15, (%rax)
;;       addq    $0x40, %rsp
;;       popq    %rbp
;;       retq
;;  130: ud2
;;  132: addb    %al, (%rax)
;;  134: addb    %al, (%rax)
;;  136: addb    %al, (%rax)
;;  138: addb    %al, (%rax)
;;  13a: addb    %al, (%rax)
;;  13c: addb    %al, (%rax)
;;  13e: addb    %al, (%rax)
;;  140: addb    %al, (%rax)
;;  142: addb    %al, (%rax)
;;  144: addb    %al, (%rax)
;;  146: addb    %al, (%rax)
