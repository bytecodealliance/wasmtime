;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_popcnt", "-Ccranelift-has_sse42"]

(module
    (func (param i32) (result i32)
      local.get 0
      i32.popcnt
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3f
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       popcntl %eax, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   3f: ud2
