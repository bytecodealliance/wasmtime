;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-Ccollector=drc"

(module
  (global $g (mut externref) (ref.null extern))
  (func (export "get") (result externref)
    (global.get $g)))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x104
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    0x30(%r14), %eax
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xf1
;;   48: movl    %eax, %ecx
;;       andl    $1, %ecx
;;       testl   %ecx, %ecx
;;       jne     0xf1
;;   58: movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rax, %rbx
;;       addq    $0x14, %rbx
;;       cmpq    %rdx, %rbx
;;       ja      0x106
;;   77: movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    (%rdx), %ebx
;;       andl    $2, %ebx
;;       testl   %ebx, %ebx
;;       jne     0xf1
;;   8d: movq    0x20(%r14), %rsi
;;       movl    (%rsi), %ebx
;;       movl    %ebx, 0x10(%rdx)
;;       movl    (%rdx), %ebx
;;       orl     $2, %ebx
;;       movl    %ebx, (%rdx)
;;       movq    8(%rdx), %rbx
;;       addq    $1, %rbx
;;       movq    %rbx, 8(%rdx)
;;       movl    %eax, (%rsi)
;;       movl    4(%rsi), %ebx
;;       addl    $1, %ebx
;;       movl    %ebx, 4(%rsi)
;;       movl    8(%rsi), %edx
;;       addl    %edx, %edx
;;       cmpl    %edx, %ebx
;;       jb      0xf1
;;   ca: cmpl    $0x400, %ebx
;;       jb      0xf1
;;   d6: subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       callq   0x20c
;;       addq    $0xc, %rsp
;;       ╰─╼ stack_map: frame_size=32, frame_offsets=[12]
;;       movq    0xc(%rsp), %r14
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  104: ud2
;;  106: ud2
