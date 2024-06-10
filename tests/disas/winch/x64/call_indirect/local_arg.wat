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
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x36
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   36: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x126
;;   5b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    $0, %ecx
;;       movq    %r14, %rdx
;;       movl    0x60(%rdx), %ebx
;;       cmpl    %ebx, %ecx
;;       jae     0x128
;;   94: movl    %ecx, %r11d
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0xdc
;;   b7: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    (%rsp), %edx
;;       callq   0x303
;;       addq    $4, %rsp
;;       movq    0x14(%rsp), %r14
;;       jmp     0xe0
;;   dc: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0x12a
;;   e9: movq    0x50(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0x12c
;;   fb: movq    0x18(%rax), %rbx
;;       movq    8(%rax), %rcx
;;       subq    $4, %rsp
;;       movq    %rbx, %rdi
;;       movq    %r14, %rsi
;;       movl    4(%rsp), %edx
;;       callq   *%rcx
;;       addq    $4, %rsp
;;       addq    $4, %rsp
;;       movq    0x10(%rsp), %r14
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;  126: ud2
;;  128: ud2
;;  12a: ud2
;;  12c: ud2
