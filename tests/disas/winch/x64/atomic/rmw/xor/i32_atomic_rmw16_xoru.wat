;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i32)
        (i32.atomic.rmw16.xor_u (i32.const 0) (i32.const 42))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x14, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x80
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0x2a, %eax
;;       movl    $0, %ecx
;;       andw    $1, %cx
;;       cmpw    $0, %cx
;;       jne     0x82
;;   44: movl    $0, %ecx
;;       movq    0x58(%r14), %r11
;;       movq    (%r11), %rdx
;;       addq    %rcx, %rdx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movzwq  (%rdx), %rax
;;       movq    %rax, %r11
;;       xorq    %rcx, %r11
;;       lock cmpxchgw %r11w, (%rdx)
;;       jne     0x65
;;   77: movzwl  %ax, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   80: ud2
;;   82: ud2
