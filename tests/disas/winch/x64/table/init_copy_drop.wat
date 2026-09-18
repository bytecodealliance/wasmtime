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
;;       ja      0xa0e
;;  15c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       callq   0x111c
;;       movq    8(%rsp), %r14
;;       pushq   %rax
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       callq   0x1147
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rcx
;;       popq    %rdx
;;       movl    $4, %ebx
;;       movl    $0, %esi
;;       movl    $7, %edi
;;       movl    %ebx, %ebx
;;       movl    %esi, %r8d
;;       addl    %ebx, %r8d
;;       jb      0xa10
;;  1ca: cmpl    %edx, %r8d
;;       ja      0xa12
;;  1d3: movl    %edi, %r8d
;;       addl    %ebx, %r8d
;;       jb      0xa14
;;  1df: cmpl    %ecx, %r8d
;;       ja      0xa16
;;  1e8: movl    %esi, %esi
;;       imulq   $0x10, %rsi, %rsi
;;       addq    %rsi, %rax
;;       cmpq    $0, %rbx
;;       je      0x256
;;  1fe: movq    (%rax), %rcx
;;       addq    $0x10, %rax
;;       movl    %edi, %edx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %r8
;;       cmpq    %r8, %rdx
;;       jae     0xa18
;;  21c: movq    %rdx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r9
;;       addq    %r11, %rsi
;;       cmpq    %r8, %rdx
;;       cmovaeq %r9, %rsi
;;       orq     $1, %rcx
;;       movq    %rcx, (%rsi)
;;       addl    $1, %edi
;;       subq    $1, %rbx
;;       jmp     0x1f4
;;  256: movq    %r14, %rdi
;;       movl    $0, %esi
;;       callq   0x1172
;;       movq    8(%rsp), %r14
;;       movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0x111c
;;       movq    8(%rsp), %r14
;;       pushq   %rax
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0x1147
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rcx
;;       popq    %rdx
;;       movl    $3, %ebx
;;       movl    $1, %esi
;;       movl    $0xf, %edi
;;       movl    %ebx, %ebx
;;       movl    %esi, %r8d
;;       addl    %ebx, %r8d
;;       jb      0xa1a
;;  2c3: cmpl    %edx, %r8d
;;       ja      0xa1c
;;  2cc: movl    %edi, %r8d
;;       addl    %ebx, %r8d
;;       jb      0xa1e
;;  2d8: cmpl    %ecx, %r8d
;;       ja      0xa20
;;  2e1: movl    %esi, %esi
;;       imulq   $0x10, %rsi, %rsi
;;       addq    %rsi, %rax
;;       cmpq    $0, %rbx
;;       je      0x34f
;;  2f7: movq    (%rax), %rcx
;;       addq    $0x10, %rax
;;       movl    %edi, %edx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %r8
;;       cmpq    %r8, %rdx
;;       jae     0xa22
;;  315: movq    %rdx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rsi), %rsi
;;       movq    %rsi, %r9
;;       addq    %r11, %rsi
;;       cmpq    %r8, %rdx
;;       cmovaeq %r9, %rsi
;;       orq     $1, %rcx
;;       movq    %rcx, (%rsi)
;;       addl    $1, %edi
;;       subq    $1, %rbx
;;       jmp     0x2ed
;;  34f: movq    %r14, %rdi
;;       movl    $1, %esi
;;       callq   0x1172
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
;;       jb      0xa24
;;  38c: cmpq    %rbx, %rsi
;;       ja      0xa26
;;  395: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa28
;;  3ab: cmpq    %rbx, %rsi
;;       ja      0xa2a
;;  3b4: cmpq    %rcx, %rdx
;;       jbe     0x3dd
;;  3bd: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x3e2
;;  3dd: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x4b5
;;  3ec: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa2c
;;  408: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x45d
;;  432: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x11e1
;;       addq    $0x10, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x463
;;  45d: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa2e
;;  47b: movq    %rbx, %r11
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
;;       jmp     0x3e2
;;  4b5: movl    $1, %eax
;;       movl    $0x1d, %ecx
;;       movl    $0x15, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa30
;;  4e0: cmpq    %rbx, %rsi
;;       ja      0xa32
;;  4e9: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa34
;;  4ff: cmpq    %rbx, %rsi
;;       ja      0xa36
;;  508: cmpq    %rcx, %rdx
;;       jbe     0x531
;;  511: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x536
;;  531: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x609
;;  540: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa38
;;  55c: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x5b1
;;  586: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x11e1
;;       addq    $0x10, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x5b7
;;  5b1: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa3a
;;  5cf: movq    %rbx, %r11
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
;;       jmp     0x536
;;  609: movl    $1, %eax
;;       movl    $0xa, %ecx
;;       movl    $0x18, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa3c
;;  634: cmpq    %rbx, %rsi
;;       ja      0xa3e
;;  63d: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa40
;;  653: cmpq    %rbx, %rsi
;;       ja      0xa42
;;  65c: cmpq    %rcx, %rdx
;;       jbe     0x685
;;  665: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x68a
;;  685: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x75d
;;  694: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa44
;;  6b0: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x705
;;  6da: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x11e1
;;       addq    $0x10, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x70b
;;  705: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa46
;;  723: movq    %rbx, %r11
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
;;       jmp     0x68a
;;  75d: movl    $4, %eax
;;       movl    $0xb, %ecx
;;       movl    $0xd, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa48
;;  788: cmpq    %rbx, %rsi
;;       ja      0xa4a
;;  791: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa4c
;;  7a7: cmpq    %rbx, %rsi
;;       ja      0xa4e
;;  7b0: cmpq    %rcx, %rdx
;;       jbe     0x7d9
;;  7b9: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x7de
;;  7d9: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0x8b1
;;  7e8: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa50
;;  804: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x859
;;  82e: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x11e1
;;       addq    $0x10, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x85f
;;  859: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa52
;;  877: movq    %rbx, %r11
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
;;       jmp     0x7de
;;  8b1: movl    $5, %eax
;;       movl    $0x14, %ecx
;;       movl    $0x13, %edx
;;       movl    %eax, %eax
;;       movl    %ecx, %ecx
;;       movl    %edx, %edx
;;       movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rcx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa54
;;  8dc: cmpq    %rbx, %rsi
;;       ja      0xa56
;;  8e5: movq    %r14, %r11
;;       movq    0xd8(%r11), %rbx
;;       movq    %rdx, %rsi
;;       addq    %rax, %rsi
;;       jb      0xa58
;;  8fb: cmpq    %rbx, %rsi
;;       ja      0xa5a
;;  904: cmpq    %rcx, %rdx
;;       jbe     0x92d
;;  90d: movq    $18446744073709551615, %rbx
;;       addq    %rax, %rcx
;;       subq    $1, %rcx
;;       addq    %rax, %rdx
;;       subq    $1, %rdx
;;       jmp     0x932
;;  92d: movl    $1, %ebx
;;       cmpq    $0, %rax
;;       je      0xa05
;;  93c: movq    %rcx, %rsi
;;       pushq   %rbx
;;       pushq   %rax
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rsi
;;       popq    %rcx
;;       movq    %r14, %rdx
;;       movq    0xd8(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xa5c
;;  958: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x9ad
;;  982: pushq   %rcx
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    8(%rsp), %rdx
;;       callq   0x11e1
;;       addq    $0x10, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x9b3
;;  9ad: andq    $0xfffffffffffffffe, %rax
;;       popq    %rcx
;;       popq    %rdx
;;       movq    %rdx, %rbx
;;       movq    %r14, %rsi
;;       movq    0xd8(%rsi), %rdi
;;       cmpq    %rdi, %rbx
;;       jae     0xa5e
;;  9cb: movq    %rbx, %r11
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
;;       jmp     0x932
;;  a05: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  a0e: ud2
;;  a10: ud2
;;  a12: ud2
;;  a14: ud2
;;  a16: ud2
;;  a18: ud2
;;  a1a: ud2
;;  a1c: ud2
;;  a1e: ud2
;;  a20: ud2
;;  a22: ud2
;;  a24: ud2
;;  a26: ud2
;;  a28: ud2
;;  a2a: ud2
;;  a2c: ud2
;;  a2e: ud2
;;  a30: ud2
;;  a32: ud2
;;  a34: ud2
;;  a36: ud2
;;  a38: ud2
;;  a3a: ud2
;;  a3c: ud2
;;  a3e: ud2
;;  a40: ud2
;;  a42: ud2
;;  a44: ud2
;;  a46: ud2
;;  a48: ud2
;;  a4a: ud2
;;  a4c: ud2
;;  a4e: ud2
;;  a50: ud2
;;  a52: ud2
;;  a54: ud2
;;  a56: ud2
;;  a58: ud2
;;  a5a: ud2
;;  a5c: ud2
;;  a5e: ud2
;;
;; wasm[0]::function[11]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xb5f
;;  a7c: movq    %rdi, %r14
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
;;       jae     0xb61
;;  ac1: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0xd0(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0xb1e
;;  aeb: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    0xc(%rsp), %edx
;;       callq   0x11e1
;;       addq    $0x10, %rsp
;;       movq    0x18(%rsp), %r14
;;       jmp     0xb24
;;  b1e: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0xb63
;;  b2d: movq    0x28(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0xb65
;;  b3f: pushq   %rax
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
;;  b5f: ud2
;;  b61: ud2
;;  b63: ud2
;;  b65: ud2
