;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i64)
        (i64.atomic.rmw.or (i32.const 0) (i64.const 42))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x78
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0x2a, %eax
;;       movl    $0, %ecx
;;       andq    $7, %rcx
;;       cmpq    $0, %rcx
;;       jne     0x7a
;;   4a: movl    $0, %ecx
;;       movq    0x30(%r14), %r11
;;       movq    (%r11), %rdx
;;       addq    %rcx, %rdx
;;       pushq   %rax
;;       popq    %rcx
;;       movq    (%rdx), %rax
;;       movq    %rax, %r11
;;       orq     %rcx, %r11
;;       lock cmpxchgq %r11, (%rdx)
;;       jne     0x5e
;;   6f: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   78: ud2
;;   7a: ud2
