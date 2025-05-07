;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "e") (result i64 f64)
    call $a
    block $block (result i64 f64)
      call $b
      i32.const 1
      br_table $block 1 $block
      unreachable
    end
    unreachable
  )
  (func $a (result i64 f64)
    i64.const 4
    f64.const 5
  )
  (func $b (result i64 f64)
    i64.const 7
    f64.const 8
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rsi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x40, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x108
;;   1c: movq    %rsi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rsi, 0x18(%rsp)
;;       movq    %rdx, 0x10(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       subq    $8, %rsp
;;       subq    $8, %rsp
;;       movq    %r14, %rsi
;;       movq    %r14, %rdx
;;       leaq    8(%rsp), %rdi
;;       callq   0x110
;;       addq    $8, %rsp
;;       movq    0x20(%rsp), %r14
;;       subq    $8, %rsp
;;       movsd   %xmm0, (%rsp)
;;       subq    $8, %rsp
;;       subq    $8, %rsp
;;       movq    %r14, %rsi
;;       movq    %r14, %rdx
;;       leaq    8(%rsp), %rdi
;;       callq   0x180
;;       addq    $8, %rsp
;;       movq    0x30(%rsp), %r14
;;       subq    $8, %rsp
;;       movsd   %xmm0, (%rsp)
;;       movl    $1, %eax
;;       movsd   (%rsp), %xmm0
;;       addq    $8, %rsp
;;       movl    $2, %ecx
;;       cmpl    %eax, %ecx
;;       cmovbl  %ecx, %eax
;;       leaq    0xa(%rip), %r11
;;       movslq  (%r11, %rax, 4), %rcx
;;       addq    %rcx, %r11
;;       jmpq    *%r11
;;   cd: addb    %al, %es:(%rax)
;;       addb    %dl, (%rcx)
;;       addb    %al, (%rax)
;;       addb    %ah, (%rsi)
;;       addb    %al, (%rax)
;;       addb    %ch, %cl
;;       adcl    $0x4c000000, %eax
;;       movl    (%rsp), %ebx
;;       movq    %r11, 0x10(%rsp)
;;       addq    $0x10, %rsp
;;       jmp     0xf5
;;   f3: ud2
;;       movq    0x10(%rsp), %rax
;;       popq    %r11
;;       movq    %r11, (%rax)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  108: ud2
;;
;; wasm[0]::function[1]::a:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rsi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x28, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x16f
;;  12c: movq    %rsi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rsi, 0x18(%rsp)
;;       movq    %rdx, 0x10(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movsd   0x2b(%rip), %xmm0
;;       subq    $8, %rsp
;;       movq    $4, (%rsp)
;;       movq    0x10(%rsp), %rax
;;       popq    %r11
;;       movq    %r11, (%rax)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  16f: ud2
;;  171: addb    %al, (%rax)
;;  173: addb    %al, (%rax)
;;  175: addb    %al, (%rax)
;;  177: addb    %al, (%rax)
;;  179: addb    %al, (%rax)
;;  17b: addb    %al, (%rax)
;;  17d: addb    %dl, (%rax, %rax, 2)
;;
;; wasm[0]::function[2]::b:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rsi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x28, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x1df
;;  19c: movq    %rsi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rsi, 0x18(%rsp)
;;       movq    %rdx, 0x10(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movsd   0x2b(%rip), %xmm0
;;       subq    $8, %rsp
;;       movq    $7, (%rsp)
;;       movq    0x10(%rsp), %rax
;;       popq    %r11
;;       movq    %r11, (%rax)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  1df: ud2
;;  1e1: addb    %al, (%rax)
;;  1e3: addb    %al, (%rax)
;;  1e5: addb    %al, (%rax)
;;  1e7: addb    %al, (%rax)
;;  1e9: addb    %al, (%rax)
;;  1eb: addb    %al, (%rax)
;;  1ed: addb    %ah, (%rax)
