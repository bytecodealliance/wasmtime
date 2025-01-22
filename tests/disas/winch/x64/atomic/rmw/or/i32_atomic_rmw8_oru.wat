;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i32)
        (i32.atomic.rmw8.or_u (i32.const 0) (i32.const 42))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5e
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0x2a, %ecx
;;       movl    $0, %edx
;;       movq    0x58(%r14), %r11
;;       movq    (%r11), %rbx
;;       addq    %rdx, %rbx
;;       movzbq  (%rbx), %rax
;;       movq    %rax, %r11
;;       orq     %rcx, %r11
;;       lock cmpxchgb %r11b, (%rbx)
;;       jne     0x44
;;   55: movzbl  %al, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5e: ud2
