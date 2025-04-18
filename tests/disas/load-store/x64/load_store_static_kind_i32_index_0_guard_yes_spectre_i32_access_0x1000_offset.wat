;;! target = "x86_64"
;;! test = "compile"
;;! flags = " -C cranelift-enable-heap-access-spectre-mitigation -O static-memory-forced -O static-memory-guard-size=0 -O dynamic-memory-guard-size=0"

;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
;; !!! GENERATED BY 'make-load-store-tests.sh' DO NOT EDIT !!!
;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

(module
  (memory i32 1)

  (func (export "do_store") (param i32 i32)
    local.get 0
    local.get 1
    i32.store offset=0x1000)

  (func (export "do_load") (param i32) (result i32)
    local.get 0
    i32.load offset=0x1000))

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %esi
;;       xorq    %r11, %r11
;;       movq    0x40(%rdi), %rdi
;;       leaq    0x1000(%rdi, %rsi), %r10
;;       cmpq    0xc(%rip), %rsi
;;       cmovaq  %r11, %r10
;;       movl    %ecx, (%r10)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   28: cld
;;   29: outl    %eax, %dx
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %esi
;;       xorq    %r11, %r11
;;       movq    0x40(%rdi), %rdi
;;       leaq    0x1000(%rdi, %rsi), %r10
;;       cmpq    0xc(%rip), %rsi
;;       cmovaq  %r11, %r10
;;       movl    (%r10), %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   68: cld
;;   69: outl    %eax, %dx
