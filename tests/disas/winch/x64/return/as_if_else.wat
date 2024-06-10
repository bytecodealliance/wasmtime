;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)

  (func (export "as-if-else") (param i32 i32) (result i32)
   (if (result i32)
    (local.get 0) (then (local.get 1)) (else (return (i32.const 4)))
   )
  )
)
  
;; wasm[0]::function[0]::dummy:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x31
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   31: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x92
;;   5b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    4(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x87
;;   7f: movl    (%rsp), %eax
;;       jmp     0x8c
;;   87: movl    $4, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   92: ud2
