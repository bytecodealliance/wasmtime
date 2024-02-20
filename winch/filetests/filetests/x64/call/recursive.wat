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
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f878b000000         	ja	0xa6
;;   1b:	 4883ec10             	sub	rsp, 0x10
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 83f801               	cmp	eax, 1
;;      	 b800000000           	mov	eax, 0
;;      	 400f9ec0             	setle	al
;;      	 85c0                 	test	eax, eax
;;      	 0f8409000000         	je	0x48
;;   3f:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 e958000000           	jmp	0xa0
;;   48:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 83e801               	sub	eax, 1
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec0c             	sub	rsp, 0xc
;;      	 8b7c240c             	mov	edi, dword ptr [rsp + 0xc]
;;      	 e800000000           	call	0x63
;;      	 4883c40c             	add	rsp, 0xc
;;      	 4883c404             	add	rsp, 4
;;      	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;      	 83e902               	sub	ecx, 2
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4883ec08             	sub	rsp, 8
;;      	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;      	 e800000000           	call	0x8d
;;      	 4883c408             	add	rsp, 8
;;      	 4883c404             	add	rsp, 4
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 01c1                 	add	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   a6:	 0f0b                 	ud2	
