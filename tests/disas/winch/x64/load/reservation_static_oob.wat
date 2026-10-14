;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Omemory-reservation=0x20000", "-Omemory-may-move=n"]

;; The memory's minimum size fits within `memory-reservation` and the memory
;; can't move, so the reservation is an upper bound: an access whose offset
;; exceeds it is out of bounds and must trap, via the regular bounds check.
(module
  (memory 2)
  (func (export "load_offset") (param i32) (result i32)
    local.get 0
    i32.load offset=0x20000))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7b
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       movq    0x40(%r14), %rcx
;;       movl    %eax, %edx
;;       addq    $0x20004, %rdx
;;       jb      0x7d
;;   4b: cmpq    %rcx, %rdx
;;       ja      0x7f
;;   54: movq    0x38(%r14), %rbx
;;       movl    %eax, %eax
;;       addq    %rax, %rbx
;;       addq    $0x20000, %rbx
;;       movl    $0, %esi
;;       cmpq    %rcx, %rdx
;;       cmovaq  %rsi, %rbx
;;       movl    (%rbx), %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   7b: ud2
;;   7d: ud2
;;   7f: ud2
