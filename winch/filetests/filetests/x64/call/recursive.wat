;;! target = "x86_64"

(module
  (func $fibonacci8 (param $n i32) (result i32)
    (if (result i32) (i32.le_s (local.get $n) (i32.const 1))
      (then
        ;; If n <= 1, return n (base case)
        (local.get $n)
      )
      (else
        ;; Else, return fibonacci(n - 1) + fibonacci(n - 2)
        (i32.add
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 1)) ;; Calculate n - 1
          )
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 2)) ;; Calculate n - 2
          )
        )
      )
    )
  )
  (export "fib" (func $fibonacci8))
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 83f801               	cmp	eax, 1
;;   17:	 b800000000           	mov	eax, 0
;;   1c:	 400f9ec0             	setle	al
;;   20:	 85c0                 	test	eax, eax
;;   22:	 0f8409000000         	je	0x31
;;   28:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   2c:	 e933000000           	jmp	0x64
;;   31:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   35:	 83e801               	sub	eax, 1
;;   38:	 50                   	push	rax
;;   39:	 4883ec08             	sub	rsp, 8
;;   3d:	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;   41:	 e800000000           	call	0x46
;;   46:	 4883c410             	add	rsp, 0x10
;;   4a:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   4e:	 83e902               	sub	ecx, 2
;;   51:	 50                   	push	rax
;;   52:	 51                   	push	rcx
;;   53:	 8b3c24               	mov	edi, dword ptr [rsp]
;;   56:	 e800000000           	call	0x5b
;;   5b:	 4883c408             	add	rsp, 8
;;   5f:	 59                   	pop	rcx
;;   60:	 01c1                 	add	ecx, eax
;;   62:	 89c8                 	mov	eax, ecx
;;   64:	 4883c410             	add	rsp, 0x10
;;   68:	 5d                   	pop	rbp
;;   69:	 c3                   	ret	
