;;! target = "x86_64"
;;! test = "winch"

(module
  (type (func (result i32)))  ;; type #0
  (import "a" "ef0" (func (result i32)))    ;; index 0
  (import "a" "ef1" (func (result i32)))
  (import "a" "ef2" (func (result i32)))
  (import "a" "ef3" (func (result i32)))
  (import "a" "ef4" (func (result i32)))    ;; index 4
  (table $t0 30 30 funcref)
  (table $t1 30 30 funcref)
  (elem (table $t0) (i32.const 2) func 3 1 4 1)
  (elem funcref
    (ref.func 2) (ref.func 7) (ref.func 1) (ref.func 8))
  (elem (table $t0) (i32.const 12) func 7 5 2 3 6)
  (elem funcref
    (ref.func 5) (ref.func 9) (ref.func 2) (ref.func 7) (ref.func 6))
  (func (result i32) (i32.const 5))  ;; index 5
  (func (result i32) (i32.const 6))
  (func (result i32) (i32.const 7))
  (func (result i32) (i32.const 8))
  (func (result i32) (i32.const 9))  ;; index 9
  (func (export "test")
    (table.init $t0 1 (i32.const 7) (i32.const 0) (i32.const 4))
         (elem.drop 1)
         (table.init $t0 3 (i32.const 15) (i32.const 1) (i32.const 3))
         (elem.drop 3)
         (table.copy $t0 0 (i32.const 20) (i32.const 15) (i32.const 5))
         (table.copy $t0 0 (i32.const 21) (i32.const 29) (i32.const 1))
         (table.copy $t0 0 (i32.const 24) (i32.const 10) (i32.const 1))
         (table.copy $t0 0 (i32.const 13) (i32.const 11) (i32.const 4))
         (table.copy $t0 0 (i32.const 19) (i32.const 20) (i32.const 5)))
  (func (export "check") (param i32) (result i32)
    (call_indirect $t0 (type 0) (local.get 0)))
)
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3d
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $5, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3d: ud2
;;
;; wasm[0]::function[6]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7d
;;   5c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $6, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   7d: ud2
;;
;; wasm[0]::function[7]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xbd
;;   9c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $7, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   bd: ud2
;;
;; wasm[0]::function[8]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xfd
;;   dc: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $8, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   fd: ud2
;;
;; wasm[0]::function[9]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x13d
;;  11c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $9, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  13d: ud2
;;
;; wasm[0]::function[10]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x40, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x8b3
;;  15c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    $1, %edx
;;       movl    $7, %ecx
;;       movl    $0, %r8d
;;       movl    $4, %r9d
;;       callq   0xf2f
;;       movq    8(%rsp), %r14
;;       movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0xf7a
;;       movq    8(%rsp), %r14
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    $3, %edx
;;       movl    $0xf, %ecx
;;       movl    $1, %r8d
;;       movl    $3, %r9d
;;       callq   0xf2f
;;       movq    8(%rsp), %r14
;;       movq    %r14, %rdi
;;       movl    $3, %esi
;;       callq   0xf7a
;;       movq    8(%rsp), %r14
;;       movl    $5, %eax
;;       movl    $0xf, %ecx
;;       movl    $0x14, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8b5
;;  20e: cmpq    %rbx, %rsi
;;       ja      0x8b7
;;  217: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8b9
;;  22d: cmpq    %rbx, %rsi
;;       ja      0x8bb
;;  236: cmpq    %rcx, %rdx
;;       jbe     0x25f
;;  23f: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x264
;;  25f: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x33e
;;  26e: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x8bd
;;  28a: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x2e6
;;  2b4: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0xfc2
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x2ec
;;  2e6: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0x8bf
;;  304: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x264
;;  33e: movl    $1, %eax
;;       movl    $0x1d, %ecx
;;       movl    $0x15, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8c1
;;  369: cmpq    %rbx, %rsi
;;       ja      0x8c3
;;  372: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8c5
;;  388: cmpq    %rbx, %rsi
;;       ja      0x8c7
;;  391: cmpq    %rcx, %rdx
;;       jbe     0x3ba
;;  39a: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x3bf
;;  3ba: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x499
;;  3c9: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x8c9
;;  3e5: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x441
;;  40f: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0xfc2
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x447
;;  441: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0x8cb
;;  45f: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x3bf
;;  499: movl    $1, %eax
;;       movl    $0xa, %ecx
;;       movl    $0x18, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8cd
;;  4c4: cmpq    %rbx, %rsi
;;       ja      0x8cf
;;  4cd: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8d1
;;  4e3: cmpq    %rbx, %rsi
;;       ja      0x8d3
;;  4ec: cmpq    %rcx, %rdx
;;       jbe     0x515
;;  4f5: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x51a
;;  515: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x5f4
;;  524: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x8d5
;;  540: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x59c
;;  56a: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0xfc2
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x5a2
;;  59c: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0x8d7
;;  5ba: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x51a
;;  5f4: movl    $4, %eax
;;       movl    $0xb, %ecx
;;       movl    $0xd, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8d9
;;  61f: cmpq    %rbx, %rsi
;;       ja      0x8db
;;  628: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8dd
;;  63e: cmpq    %rbx, %rsi
;;       ja      0x8df
;;  647: cmpq    %rcx, %rdx
;;       jbe     0x670
;;  650: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x675
;;  670: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x74f
;;  67f: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x8e1
;;  69b: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x6f7
;;  6c5: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0xfc2
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x6fd
;;  6f7: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0x8e3
;;  715: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x675
;;  74f: movl    $5, %eax
;;       movl    $0x14, %ecx
;;       movl    $0x13, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8e5
;;  77a: cmpq    %rbx, %rsi
;;       ja      0x8e7
;;  783: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0x8e9
;;  799: cmpq    %rbx, %rsi
;;       ja      0x8eb
;;  7a2: cmpq    %rcx, %rdx
;;       jbe     0x7cb
;;  7ab: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x7d0
;;  7cb: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x8aa
;;  7da: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x8ed
;;  7f6: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x852
;;  820: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0xfc2
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x858
;;  852: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0x8ef
;;  870: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r8
;;       addq    %r11, %rsi
;;       cmpq    %rdi, %rbx
;;       cmovaeq %r8, %rsi
;;       orq     $1, %rax
;;       movq    %rax, (%rsi)
;;       popq    %rax
;;       popq    %rbx
;;       addq    %rbx, %rdx
;;       addq    %rbx, %rcx
;;       subq    $1, %rax
;;       jmp     0x7d0
;;  8aa: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  8b3: ud2
;;  8b5: ud2
;;  8b7: ud2
;;  8b9: ud2
;;  8bb: ud2
;;  8bd: ud2
;;  8bf: ud2
;;  8c1: ud2
;;  8c3: ud2
;;  8c5: ud2
;;  8c7: ud2
;;  8c9: ud2
;;  8cb: ud2
;;  8cd: ud2
;;  8cf: ud2
;;  8d1: ud2
;;  8d3: ud2
;;  8d5: ud2
;;  8d7: ud2
;;  8d9: ud2
;;  8db: ud2
;;  8dd: ud2
;;  8df: ud2
;;  8e1: ud2
;;  8e3: ud2
;;  8e5: ud2
;;  8e7: ud2
;;  8e9: ud2
;;  8eb: ud2
;;  8ed: ud2
;;  8ef: ud2
;;
;; wasm[0]::function[11]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xa06
;;  91c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa08
;;  961: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x9c5
;;  98b: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    0xc(%rsp), %edx
;;       callq   0xfc2
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       jmp     0x9cb
;;  9c5: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0xa0a
;;  9d4: movq    0x28(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0xa0c
;;  9e6: pushq   %rax
;;       popq    %rcx
;;       movq    0x18(%rcx), %rbx
;;       movq    8(%rcx), %rdx
;;       movq    %rbx, %rdi
;;       movq    %r14, %rsi
;;       callq   *%rdx
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  a06: ud2
;;  a08: ud2
;;  a0a: ud2
;;  a0c: ud2
