;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i64)
        (i64.atomic.rmw.and (i32.const 0) (i64.const 42))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $0x2a, %rax
;;       movl    $0, %ecx
;;       andq    $7, %rcx
;;       cmpq    $0, %rcx
;;       jne     0x7c
;;   4c: movl    $0, %ecx
;;       movq    0x38(%r14), %r11
;;       movq    (%r11), %rdx
;;       addq    %rcx, %rdx
;;       pushq   %rax
;;       popq    %rcx
;;       movq    (%rdx), %rax
;;       movq    %rax, %r11
;;       andq    %rcx, %r11
;;       lock cmpxchgq %r11, (%rdx)
;;       jne     0x60
;;   71: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   7a: ud2
;;   7c: ud2
