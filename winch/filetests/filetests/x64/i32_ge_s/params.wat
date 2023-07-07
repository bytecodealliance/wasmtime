;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
        (local.get 0)
        (local.get 1)
        (i32.ge_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   18:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   1c:	 39c1                 	cmp	ecx, eax
;;   1e:	 b900000000           	mov	ecx, 0
;;   23:	 400f9dc1             	setge	cl
;;   27:	 4889c8               	mov	rax, rcx
;;   2a:	 4883c410             	add	rsp, 0x10
;;   2e:	 5d                   	pop	rbp
;;   2f:	 c3                   	ret	
