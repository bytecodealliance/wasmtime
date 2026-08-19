;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=null"

;; Store an external reference in an exception payload.
(module
  (tag $e (param externref))
  (func (param externref)
    (throw $e (local.get 0))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x40, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x189
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       callq   0x233
;;       addq    $0xc, %rsp
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[12, 28]
;;       movq    0x1c(%rsp), %r14
;;       movq    0x20(%r14), %r11
;;       movl    (%r11), %ecx
;;       addl    $7, %ecx
;;       jb      0x18b
;;   72: andl    $0xfffffff8, %ecx
;;       movl    %ecx, %edx
;;       addl    $0x18, %edx
;;       jb      0x18d
;;   86: subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $4, %rsp
;;       movl    %edx, (%rsp)
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       cmpq    %rdx, %rax
;;       jbe     0xee
;;   c3: subq    %rdx, %rax
;;       pushq   %rax
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movq    0xc(%rsp), %rsi
;;       callq   0x1ec
;;       addq    $0xc, %rsp
;;       ╰─╼ stack_map: frame_size=64, frame_offsets=[28, 44]
;;       addq    $8, %rsp
;;       movq    0x24(%rsp), %r14
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    $0x4000018, (%rdx)
;;       movq    0x28(%r14), %r11
;;       movl    4(%r11), %r11d
;;       movl    %r11d, 4(%rdx)
;;       movq    0x20(%r14), %rcx
;;       movl    %eax, %r11d
;;       addl    $0x18, %r11d
;;       movl    %r11d, (%rcx)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    %ecx, 8(%rdx)
;;       movl    $0, 0xc(%rdx)
;;       movl    (%rsp), %r11d
;;       addq    $4, %rsp
;;       movl    %r11d, 0x10(%rdx)
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x260
;;       addq    $0xc, %rsp
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[28]
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  189: ud2
;;  18b: ud2
;;  18d: ud2
