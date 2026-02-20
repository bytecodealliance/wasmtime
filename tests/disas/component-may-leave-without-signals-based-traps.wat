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
;;       movq    %rsi, %rbx
;;       movq    %rdx, %r8
;;       movq    0x10(%rdi), %rax
;;       movq    %rbp, %rcx
;;       movq    %rcx, 0x30(%rax)
;;       movq    %rbp, %rcx
;;       movq    8(%rcx), %rcx
;;       movq    %rcx, 0x38(%rax)
;;       movl    0x20(%rdi), %eax
;;       testl   $1, %eax
;;       je      0x13e
;;   fe: movq    8(%rdi), %rax
;;       movq    (%rax), %rax
;;       xorl    %ecx, %ecx
;;       movl    %ecx, %esi
;;       movl    %ecx, %edx
;;       movq    %r8, %rcx
;;       movl    %ecx, %ecx
;;       callq   *%rax
;;       cmpq    $-1, %rax
;;       je      0x129
;;  11c: movq    (%rsp), %rbx
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  129: movq    %rbx, %rsi
;;  12c: movq    0x10(%rsi), %rax
;;  130: movq    0x198(%rax), %rax
;;  137: movq    %rsi, %rdi
;;  13a: callq   *%rax
;;  13c: ud2
;;  13e: movq    %rdi, %rbx
;;  141: movl    $0x17, %esi
;;  146: callq   0x6a
;;  14b: movq    %rbx, %rdi
;;  14e: callq   0x9b
;;  153: ud2
