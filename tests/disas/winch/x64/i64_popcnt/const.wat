;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_popcnt", "-Ccranelift-has_sse42"]

(module
    (func (result i64)
      i64.const 3
      i64.popcnt
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3d
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $3, %rax
;;       popcntq %rax, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3d: ud2
