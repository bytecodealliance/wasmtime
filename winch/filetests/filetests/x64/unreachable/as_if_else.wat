;;! target = "x86_64"


(module
  (func (export "as-if-else") (param i32 i32) (result i32)
    (if (result i32) (local.get 0) (then (local.get 1)) (else (unreachable)))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   18:	 85c0                 	test	eax, eax
;;   1a:	 0f8409000000         	je	0x29
;;   20:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   24:	 e902000000           	jmp	0x2b
;;   29:	 0f0b                 	ud2	
;;   2b:	 4883c410             	add	rsp, 0x10
;;   2f:	 5d                   	pop	rbp
;;   30:	 c3                   	ret	
