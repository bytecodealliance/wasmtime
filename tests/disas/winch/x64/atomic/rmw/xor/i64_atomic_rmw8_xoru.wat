;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i64)
        (i64.atomic.rmw8.xor_u (i32.const 0) (i64.const 42))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x63
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $0x2a, %rax
;;       movl    $0, %ecx
;;       movq    0x48(%r14), %r11
;;       movq    (%r11), %rdx
;;       addq    %rcx, %rdx
;;       pushq   %rax
;;       popq    %rcx
;;       movzbq  (%rdx), %rax
;;       movq    %rax, %r11
;;       xorq    %rcx, %r11
;;       lock cmpxchgb %r11b, (%rdx)
;;       jne     0x48
;;   59: movzbq  %al, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   63: ud2
