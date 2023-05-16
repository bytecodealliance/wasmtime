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
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;   18:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   1c:	 31d2                 	xor	edx, edx
;;   1e:	 f7f1                 	div	ecx
;;   20:	 4889d0               	mov	rax, rdx
;;   23:	 4883c410             	add	rsp, 0x10
;;   27:	 5d                   	pop	rbp
;;   28:	 c3                   	ret	
