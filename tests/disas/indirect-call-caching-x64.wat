;;! target = "x86_64"
;;! test = "compile"
;;! flags = [ "-Ocache-call-indirects=y" ]

;; This test checks that we get the indirect-call caching optimization
;; where it should be applicable (immutable table, null 0-index).
;;
;; Here we're testing that the array accesses lower into reasonable address
;; modes; the core bit to check for a cache hit (callee index in edx) is:
;;
;;       movq    %rdx, %rbx
;;       andl    $0x3ff, %ebx                 ;; masking for a 1024-entry cache
;;       movl    0xf0(%rdi, %rbx, 4), %ecx    ;; cache tag (index)
;;       movq    0x10f0(%rdi, %rbx, 8), %r8   ;; cache value (raw code ptr)
;;       cmpl    %edx, %ecx                   ;; tag compare
;;       jne     0xe6

(module
 (table 10 10 funcref)

 (func $f1 (result i32) i32.const 1)
 (func $f2 (result i32) i32.const 2)
 (func $f3 (result i32) i32.const 3)

 (func (export "call_it") (param i32) (result i32)
  local.get 0
  call_indirect (result i32))

 (elem (i32.const 1) func $f1 $f2 $f3))
;; wasm[0]::function[0]::f1:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $1, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::f2:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $2, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]::f3:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $3, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    (%r10), %r10
;;       addq    $0x30, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x168
;;   78: subq    $0x20, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r14, 8(%rsp)
;;       movq    %r15, 0x10(%rsp)
;;       movq    %rdx, %rbx
;;       andl    $0x3ff, %ebx
;;       movl    0xf0(%rdi, %rbx, 4), %ecx
;;       movq    0x10f0(%rdi, %rbx, 8), %r8
;;       cmpl    %edx, %ecx
;;       jne     0xe6
;;   aa: movq    %rdi, %rsi
;;       movq    %rsi, %rax
;;       movq    %rax, %rdi
;;       jmp     0xcc
;;   b8: movq    0x18(%rax), %rdi
;;       movq    8(%rax), %r8
;;       cmpq    %r14, %rdi
;;       je      0x14a
;;   c9: movq    %r14, %rsi
;;       callq   *%r8
;;       movq    (%rsp), %rbx
;;       movq    8(%rsp), %r14
;;       movq    0x10(%rsp), %r15
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   e6: xorq    %r8, %r8
;;   e9: movq    0x58(%rdi), %r9
;;   ed: movq    %rdi, %r11
;;   f0: movl    %edx, %r10d
;;   f3: leaq    (%r9, %r10, 8), %rcx
;;   f7: cmpl    $0xa, %edx
;;   fa: movq    %rdx, %r15
;;   fd: cmovaeq %r8, %rcx
;;  101: movq    (%rcx), %r8
;;  104: movq    %r8, %rax
;;  107: andq    $0xfffffffffffffffe, %rax
;;  10b: testq   %r8, %r8
;;  10e: je      0x11c
;;  114: movq    %r11, %r14
;;  117: jmp     0x135
;;  11c: xorl    %r10d, %r10d
;;  11f: movl    %r10d, %esi
;;  122: movq    %r15, %rcx
;;  125: movl    %ecx, %edx
;;  127: movq    %r11, %r14
;;  12a: movq    %rcx, %r15
;;  12d: movq    %r14, %rdi
;;  130: callq   0x413
;;  135: movl    0x10(%rax), %r11d
;;  139: movq    0x50(%r14), %rsi
;;  13d: movl    (%rsi), %esi
;;  13f: cmpl    %esi, %r11d
;;  142: je      0xb8
;;  148: ud2
;;  14a: movq    %r15, %rdx
;;  14d: movl    %edx, 0xf0(%r14, %rbx, 4)
;;  155: movq    %r8, 0x10f0(%r14, %rbx, 8)
;;  15d: movq    %r14, %r11
;;  160: movq    %r11, %rsi
;;  163: jmp     0xcc
;;  168: ud2
