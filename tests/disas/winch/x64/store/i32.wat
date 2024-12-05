;;! target = "x86_64"
;;! test = "winch"
(module
  (memory 1)

  (func (export "as-block-value")
    (block (i32.store (i32.const 0) (i32.const 1)))
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x45
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $1, %eax
;;       movl    $0, %ecx
;;       movq    0x60(%r14), %rdx
;;       addq    %rcx, %rdx
;;       movl    %eax, (%rdx)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   45: ud2
