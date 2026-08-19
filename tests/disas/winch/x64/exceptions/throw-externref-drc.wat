;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=drc"

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
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x16b
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
;;       callq   0x21b
;;       addq    $0xc, %rsp
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[12, 28]
;;       movq    0x1c(%rsp), %r14
;;       movq    0x28(%r14), %rcx
;;       movl    4(%rcx), %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movl    $0x4000000, %esi
;;       movl    4(%rsp), %edx
;;       movl    $0x28, %ecx
;;       movl    $8, %r8d
;;       callq   0x1cc
;;       addq    $4, %rsp
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[12, 28]
;;       addq    $4, %rsp
;;       movq    0x20(%rsp), %r14
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    %ecx, 0x18(%rdx)
;;       movl    $0, 0x1c(%rdx)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       testl   %ecx, %ecx
;;       je      0x12f
;;   e8: movl    %ecx, %r11d
;;       andl    $1, %r11d
;;       testl   %r11d, %r11d
;;       jne     0x12f
;;   fb: movq    8(%r14), %rbx
;;       movq    0x28(%rbx), %rsi
;;       movq    0x20(%rbx), %rbx
;;       movq    %rcx, %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsi, %r11
;;       ja      0x16d
;;  11a: movq    %rbx, %rdi
;;       addq    %rcx, %rdi
;;       movq    8(%rdi), %r8
;;       addq    $1, %r8
;;       movq    %r8, 8(%rdi)
;;       movl    %ecx, 0x20(%rdx)
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x248
;;       addq    $0xc, %rsp
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[28]
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  16b: ud2
;;  16d: ud2
