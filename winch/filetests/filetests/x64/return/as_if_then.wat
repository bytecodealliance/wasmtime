;;! target = "x86_64"
(module
  (func $dummy)

  (func (export "as-if-then") (param i32 i32) (result i32)
   (if (result i32)
    (local.get 0) (then (return (i32.const 3))) (else (local.get 1))
   )
  )
)
  
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   18:	 85c0                 	test	eax, eax
;;   1a:	 0f840c000000         	je	0x2c
;;   20:	 48c7c003000000       	mov	rax, 3
;;   27:	 e904000000           	jmp	0x30
;;   2c:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   30:	 4883c410             	add	rsp, 0x10
;;   34:	 5d                   	pop	rbp
;;   35:	 c3                   	ret	
