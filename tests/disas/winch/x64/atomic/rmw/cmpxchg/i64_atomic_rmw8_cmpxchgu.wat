;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i64)
        (i64.atomic.rmw8.cmpxchg_u (i32.const 0) (i64.const 42) (i64.const 1337))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5b
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $0x539, %rax
;;       movq    $0x2a, %rcx
;;       movl    $0, %edx
;;       movq    0x58(%r14), %r11
;;       movq    (%r11), %rbx
;;       addq    %rdx, %rbx
;;       pushq   %rcx
;;       pushq   %rax
;;       popq    %rcx
;;       popq    %rax
;;       lock cmpxchgb %cl, (%rbx)
;;       movzbq  %al, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5b: ud2
