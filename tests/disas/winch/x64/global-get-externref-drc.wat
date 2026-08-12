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
;;       ja      0x112
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    0x30(%r14), %eax
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xff
;;   48: movl    %eax, %r11d
;;       andl    $1, %r11d
;;       testl   %r11d, %r11d
;;       jne     0xff
;;   5b: movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rax, %r11
;;       addq    $0x14, %r11
;;       cmpq    %rdx, %r11
;;       ja      0x114
;;   7a: movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    (%rdx), %r11d
;;       andl    $2, %r11d
;;       testl   %r11d, %r11d
;;       jne     0xff
;;   93: movq    0x20(%r14), %rbx
;;       movl    (%rbx), %r11d
;;       movl    %r11d, 0x10(%rdx)
;;       movl    (%rdx), %r11d
;;       orl     $2, %r11d
;;       movl    %r11d, (%rdx)
;;       movq    8(%rdx), %rsi
;;       addq    $1, %rsi
;;       movq    %rsi, 8(%rdx)
;;       movl    %eax, (%rbx)
;;       movl    4(%rbx), %esi
;;       addl    $1, %esi
;;       movl    %esi, 4(%rbx)
;;       movl    8(%rbx), %r11d
;;       addl    %r11d, %r11d
;;       cmpl    %r11d, %esi
;;       jb      0xff
;;   d8: cmpl    $0x400, %esi
;;       jb      0xff
;;   e4: subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       callq   0x21a
;;       addq    $0xc, %rsp
;;       ╰─╼ stack_map: frame_size=32, frame_offsets=[12]
;;       movq    0xc(%rsp), %r14
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  112: ud2
;;  114: ud2
