;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "f") (result i64)
    i32.const 0
    i64.const 0xDEADBEEF00000000
    i64.const 0x1234
    i64.atomic.rmw32.cmpxchg_u))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x28, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x78
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0x1234, %eax
;;       movabsq $16045690981097406464, %rcx
;;       movl    $0, %edx
;;       andl    $3, %edx
;;       cmpl    $0, %edx
;;       jne     0x7a
;;   52: movl    $0, %edx
;;       movq    0x30(%r14), %r11
;;       movq    (%r11), %rbx
;;       movl    %edx, %edx
;;       addq    %rdx, %rbx
;;       pushq   %rbx
;;       pushq   %rcx
;;       pushq   %rax
;;       popq    %rcx
;;       popq    %rax
;;       popq    %rdx
;;       lock cmpxchgl %ecx, (%rdx)
;;       movl    %eax, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   78: ud2
;;   7a: ud2
