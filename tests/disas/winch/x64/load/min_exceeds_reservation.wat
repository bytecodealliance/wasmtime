;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Omemory-reservation=0", "-Omemory-may-move=n"]

;; The memory's minimum size exceeds `memory-reservation`, so the runtime
;; sizes the allocation from the minimum and the reservation is not an upper
;; bound: this access must get a dynamic bounds check, not a static trap.
(module
  (memory 2)
  (func (export "load_offset") (param i32) (result i32)
    local.get 0
    i32.load offset=70000))
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
;;       addq    $0x11174, %rdx
;;       jb      0x7d
;;   4b: cmpq    %rcx, %rdx
;;       ja      0x7f
;;   54: movq    0x38(%r14), %rbx
;;       movl    %eax, %eax
;;       addq    %rax, %rbx
;;       addq    $0x11170, %rbx
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
