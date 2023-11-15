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
;;   2c:	 e958000000           	jmp	0x89
;;   31:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   35:	 83e801               	sub	eax, 1
;;   38:	 4883ec04             	sub	rsp, 4
;;   3c:	 890424               	mov	dword ptr [rsp], eax
;;   3f:	 4883ec0c             	sub	rsp, 0xc
;;   43:	 8b7c240c             	mov	edi, dword ptr [rsp + 0xc]
;;   47:	 e800000000           	call	0x4c
;;   4c:	 4883c40c             	add	rsp, 0xc
;;   50:	 4883c404             	add	rsp, 4
;;   54:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   58:	 83e902               	sub	ecx, 2
;;   5b:	 4883ec04             	sub	rsp, 4
;;   5f:	 890424               	mov	dword ptr [rsp], eax
;;   62:	 4883ec04             	sub	rsp, 4
;;   66:	 890c24               	mov	dword ptr [rsp], ecx
;;   69:	 4883ec08             	sub	rsp, 8
;;   6d:	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;   71:	 e800000000           	call	0x76
;;   76:	 4883c408             	add	rsp, 8
;;   7a:	 4883c404             	add	rsp, 4
;;   7e:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   81:	 4883c404             	add	rsp, 4
;;   85:	 01c1                 	add	ecx, eax
;;   87:	 89c8                 	mov	eax, ecx
;;   89:	 4883c410             	add	rsp, 0x10
;;   8d:	 5d                   	pop	rbp
;;   8e:	 c3                   	ret	
