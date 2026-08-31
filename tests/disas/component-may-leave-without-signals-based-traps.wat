;;! target = "x86_64-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Osignals-based-traps=n"
;;! objdump = "--funcs all --filter wasm-call-component-resource-new"

(component
  (type $r (resource (rep i32)))

  (core module $m (import "" "" (func (param i32) (result i32))))

  (core func $f (canon resource.new $r))
  (core instance $i (instantiate $m (with "" (instance (export "" (func $f))))))
)
;; component-trampolines[0]-wasm-call-component-resource-new[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x10, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %rdx, %r8
;;       movq    %rbp, %rcx
;;       movq    8(%rsi), %rax
;;       movq    %rsi, %rbx
;;       movq    %rcx, 0x30(%rax)
;;       movq    %rbp, %rcx
;;       movq    8(%rcx), %rcx
;;       movq    %rcx, 0x38(%rax)
;;       movl    0x20(%rdi), %eax
;;       testl   %eax, %eax
;;       je      0x13b
;;   fb: movq    8(%rdi), %rax
;;       movq    (%rax), %rax
;;       xorl    %ecx, %ecx
;;       movl    %ecx, %esi
;;       movl    %ecx, %edx
;;       movq    %r8, %rcx
;;       movl    %ecx, %ecx
;;       callq   *%rax
;;       cmpq    $-1, %rax
;;       je      0x126
;;  119: movq    (%rsp), %rbx
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  126: movq    %rbx, %rsi
;;  129: movq    0x10(%rsi), %rax
;;  12d: movq    0x148(%rax), %rax
;;  134: movq    %rbx, %rdi
;;  137: callq   *%rax
;;  139: ud2
;;  13b: movl    $0x17, %esi
;;  140: movq    %rbx, %rdi
;;  143: callq   0x6a
;;  148: movq    %rbx, %rdi
;;  14b: callq   0x9b
;;  150: ud2
