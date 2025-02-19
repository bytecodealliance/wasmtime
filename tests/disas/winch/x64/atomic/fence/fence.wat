;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i32)
        (i32.store (i32.const 0) (i32.const 42))
        (atomic.fence)
        (i32.load (i32.const 0))
        ))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5c
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0x2a, %eax
;;       movl    $0, %ecx
;;       movq    0x50(%r14), %r11
;;       movq    (%r11), %rdx
;;       addq    %rcx, %rdx
;;       movl    %eax, (%rdx)
;;       mfence
;;       movl    $0, %eax
;;       movq    0x50(%r14), %r11
;;       movq    (%r11), %rcx
;;       addq    %rax, %rcx
;;       movl    (%rcx), %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5c: ud2
