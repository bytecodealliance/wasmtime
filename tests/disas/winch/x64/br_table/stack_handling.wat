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
;;       movq    0x10(%r11), %r11
;;       addq    $0x24, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x82
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %r11d
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
;;   5e: sbbb    (%rax), %al
;;       addb    %al, (%rax)
;;       adcl    %eax, (%rax)
;;       addb    %al, (%rax)
;;       sbbb    (%rax), %al
;;       addb    %al, (%rax)
;;       jmp     0x78
;;   6f: addq    $4, %rsp
;;       jmp     0x7c
;;   78: addq    $4, %rsp
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   82: ud2
