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
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $0x2a, %rcx
;;       movl    $0, %edx
;;       andq    $7, %rdx
;;       cmpq    $0, %rdx
;;       jne     0x71
;;   46: movl    $0, %edx
;;       movq    0x58(%r14), %r11
;;       movq    (%r11), %rbx
;;       addq    %rdx, %rbx
;;       movq    (%rbx), %rax
;;       movq    %rax, %r11
;;       andq    %rcx, %r11
;;       lock cmpxchgq %r11, (%rbx)
;;       jne     0x58
;;   69: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   6f: ud2
;;   71: ud2
