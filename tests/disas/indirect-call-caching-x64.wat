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
;;       ja      0x1a9
;;   78: subq    $0x20, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r13, 8(%rsp)
;;       movq    %r14, 0x10(%rsp)
;;       movq    %r15, 0x18(%rsp)
;;       movq    %rdi, %r15
;;       movq    %rdx, %r14
;;       andl    $0x3ff, %r14d
;;       movl    0xf0(%rdi, %r14, 4), %r9d
;;       cmpl    %edx, %r9d
;;       jne     0x117
;;   ad: movq    %r15, %rdi
;;       movl    0x10f0(%rdi, %r14, 4), %esi
;;       movq    %rdx, %rax
;;       andl    $0x3ff, %eax
;;       movq    0x20f0(%rdi, %rax, 8), %rcx
;;       testl   %esi, %esi
;;       jne     0x117
;;   d1: testq   %rcx, %rcx
;;       je      0x117
;;   da: movq    %r15, %rdi
;;       movq    %r15, %rsi
;;       jmp     0xf9
;;   e5: movq    0x18(%rax), %rdi
;;       movq    8(%rax), %rcx
;;       cmpq    %rbx, %rdi
;;       je      0x179
;;   f6: movq    %rbx, %rsi
;;       callq   *%rcx
;;       movq    (%rsp), %rbx
;;       movq    8(%rsp), %r13
;;       movq    0x10(%rsp), %r14
;;       movq    0x18(%rsp), %r15
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  117: xorq    %r8, %r8
;;  11a: movq    %r15, %rdi
;;  11d: movq    0x58(%rdi), %r9
;;  121: movl    %edx, %r10d
;;  124: leaq    (%r9, %r10, 8), %r9
;;  128: cmpl    $0xa, %edx
;;  12b: movq    %rdx, %r13
;;  12e: cmovaeq %r8, %r9
;;  132: movq    (%r9), %r9
;;  135: movq    %r9, %rax
;;  138: andq    $0xfffffffffffffffe, %rax
;;  13c: testq   %r9, %r9
;;  13f: je      0x14d
;;  145: movq    %r15, %rbx
;;  148: jmp     0x166
;;  14d: xorl    %r11d, %r11d
;;  150: movl    %r11d, %esi
;;  153: movq    %r13, %rdi
;;  156: movl    %edi, %edx
;;  158: movq    %r15, %rbx
;;  15b: movq    %rdi, %r13
;;  15e: movq    %rbx, %rdi
;;  161: callq   0x454
;;  166: movl    0x10(%rax), %esi
;;  169: movq    0x50(%rbx), %rdi
;;  16d: movl    (%rdi), %edi
;;  16f: cmpl    %edi, %esi
;;  171: je      0xe5
;;  177: ud2
;;  179: movq    %r13, %rdx
;;  17c: movl    %edx, 0xf0(%rbx, %r14, 4)
;;  184: movl    $0, 0x10f0(%rbx, %r14, 4)
;;  190: andl    $0x3ff, %edx
;;  196: movq    %rcx, 0x20f0(%rbx, %rdx, 8)
;;  19e: movq    %rbx, %r15
;;  1a1: movq    %r15, %rsi
;;  1a4: jmp     0xf9
;;  1a9: ud2
