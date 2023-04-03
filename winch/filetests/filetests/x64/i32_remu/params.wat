;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.rem_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 897c2404             	mov	dword ptr [rsp + 4], edi
;;    c:	 893424               	mov	dword ptr [rsp], esi
;;    f:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   12:	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;   16:	 31d2                 	xor	edx, edx
;;   18:	 f7f1                 	div	ecx
;;   1a:	 4889d0               	mov	rax, rdx
;;   1d:	 4883c408             	add	rsp, 8
;;   21:	 5d                   	pop	rbp
;;   22:	 c3                   	ret	
