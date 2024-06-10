;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") 
    call 1
    call 1
    br_if 0
    drop
  )
  (func (;1;) (result i32)
    i32.const 1
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x75
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0x80
;;       movq    8(%rsp), %r14
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0x80
;;       addq    $0xc, %rsp
;;       movq    0xc(%rsp), %r14
;;       testl   %eax, %eax
;;       je      0x6b
;;   62: addq    $4, %rsp
;;       jmp     0x6f
;;   6b: addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   75: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xb6
;;   9b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $1, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   b6: ud2
