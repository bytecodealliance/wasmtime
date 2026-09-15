;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-Ccollector=drc"

(module
  (global $g (mut externref) (ref.null extern))
  (func (param externref)
    (global.set $g (local.get 0))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x129
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movl    0x30(%r14), %ebx
;;       testl   %eax, %eax
;;       je      0xa1
;;   66: movl    %eax, %r11d
;;       andl    $1, %r11d
;;       testl   %r11d, %r11d
;;       jne     0xa1
;;   79: movq    %rax, %r11
;;       addq    $0x10, %r11
;;       cmpq    %rdx, %r11
;;       ja      0x12b
;;   8c: movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       movq    8(%rsi), %rdi
;;       addq    $1, %rdi
;;       movq    %rdi, 8(%rsi)
;;       movl    %eax, 0x30(%r14)
;;       testl   %ebx, %ebx
;;       je      0x120
;;   ad: movl    %ebx, %r11d
;;       andl    $1, %r11d
;;       testl   %r11d, %r11d
;;       jne     0x120
;;   c0: movq    %rbx, %r11
;;       addq    $0x10, %r11
;;       cmpq    %rdx, %r11
;;       ja      0x12d
;;   d3: movq    %rcx, %rsi
;;       addq    %rbx, %rsi
;;       movq    8(%rsi), %rdi
;;       subq    $1, %rdi
;;       cmpq    $0, %rdi
;;       je      0xf7
;;   ee: movq    %rdi, 8(%rsi)
;;       jmp     0x120
;;   f7: subq    $4, %rsp
;;       movl    %ebx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x18c
;;       addq    $0x10, %rsp
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[28]
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  129: ud2
;;  12b: ud2
;;  12d: ud2
