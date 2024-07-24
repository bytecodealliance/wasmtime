;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-if-cond") (param i32) (result i32)
    (block (result i32)
      (if (result i32)
        (br_if 0 (i32.const 1) (local.get 0))
        (then (i32.const 2))
        (else (i32.const 3))
      )
    )
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5e
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %ecx
;;       movl    $1, %eax
;;       testl   %ecx, %ecx
;;       jne     0x58
;;   41: testl   %eax, %eax
;;       je      0x53
;;   49: movl    $2, %eax
;;       jmp     0x58
;;   53: movl    $3, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   5e: ud2
