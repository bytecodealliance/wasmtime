;;! target="x86_64"
;;! test = "winch"

(module
    (type $param-i32 (func (param i32)))

    (func $param-i32 (type $param-i32))
    (func (export "")
        (local i32)
        local.get 0
        (call_indirect (type $param-i32) (i32.const 0))
    )

    (table funcref
      (elem
        $param-i32)
    )
)

;; wasm[0]::function[0]::param-i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x37
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   37: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x133
;;   5c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    $0, %ecx
;;       movq    %r14, %rdx
;;       movq    0x60(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x135
;;   98: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0xe9
;;   bb: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    8(%rsp), %edx
;;       callq   0x35d
;;       addq    $8, %rsp
;;       addq    $4, %rsp
;;       movq    0x1c(%rsp), %r14
;;       jmp     0xed
;;   e9: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0x137
;;   f6: movq    0x50(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0x139
;;  108: movq    0x18(%rax), %rbx
;;       movq    8(%rax), %rcx
;;       subq    $0xc, %rsp
;;       movq    %rbx, %rdi
;;       movq    %r14, %rsi
;;       movl    0xc(%rsp), %edx
;;       callq   *%rcx
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  133: ud2
;;  135: ud2
;;  137: ud2
;;  139: ud2
