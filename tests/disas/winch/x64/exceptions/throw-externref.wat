;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=copying"

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
;;       ja      0x11e
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
;;       callq   0x1cc
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
;;       movl    $0x4000022, %esi
;;       movl    4(%rsp), %edx
;;       movl    $0x20, %ecx
;;       movl    $0x10, %r8d
;;       callq   0x17d
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
;;       movl    %ecx, 0x10(%rdx)
;;       movl    $0, 0x14(%rdx)
;;       movl    (%rsp), %r11d
;;       addq    $4, %rsp
;;       movl    %r11d, 0x18(%rdx)
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x1f9
;;       addq    $0xc, %rsp
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[28]
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  11e: ud2
