;;! target = "x86_64"
;;! test = "winch"
(module
  (func (;0;) (param i32)
    local.get 0
    block ;; label = @1
      i32.const 808727609
      br_table 0 (;@1;) 1 (;@0;) 0 (;@1;)
    end
    drop
  )
  (export "main" (func 0))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x1c, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x81
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    $0x30343439, %eax
;;       movl    $2, %ecx
;;       cmpl    %eax, %ecx
;;       cmovbl  %ecx, %eax
;;       leaq    0xa(%rip), %r11
;;       movslq  (%r11, %rax, 4), %rcx
;;       addq    %rcx, %r11
;;       jmpq    *%r11
;;   5d: sbbb    (%rax), %al
;;       addb    %al, (%rax)
;;       adcl    %eax, (%rax)
;;       addb    %al, (%rax)
;;       sbbb    (%rax), %al
;;       addb    %al, (%rax)
;;       jmp     0x77
;;   6e: addq    $4, %rsp
;;       jmp     0x7b
;;   77: addq    $4, %rsp
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   81: ud2
