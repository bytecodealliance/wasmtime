;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-local-set-value") (param i32) (result i32)
    (local i32)
    (block (result i32)
      (local.set 0 (br_if 0 (i32.const 17) (local.get 0)))
      (i32.const -1)
    )
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5a
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    $0, (%rsp)
;;       xorl    %r11d, %r11d
;;       movl    4(%rsp), %ecx
;;       movl    $0x11, %eax
;;       testl   %ecx, %ecx
;;       jne     0x54
;;   4b: movl    %eax, 4(%rsp)
;;       movl    $0xffffffff, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   5a: ud2
