;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i32)
        (memory.atomic.notify offset=8 (i32.const 0) (i32.const 10))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x73
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0xa, %eax
;;       movl    $0, %ecx
;;       movl    $0, %edx
;;       movl    %ecx, %ecx
;;       addq    $8, %rcx
;;       subq    $4, %rsp
;;       movl    %edx, (%rsp)
;;       pushq   %rcx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       movq    4(%rsp), %rdx
;;       movl    (%rsp), %ecx
;;       callq   0x196
;;       addq    $0x10, %rsp
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   73: ud2
