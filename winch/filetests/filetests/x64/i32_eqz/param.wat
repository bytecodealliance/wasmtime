;;! target = "x86_64"

(module
    (func (param i32) (result i32)
        (local.get 0)
        (i32.eqz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 83f800               	cmp	eax, 0
;;   17:	 b800000000           	mov	eax, 0
;;   1c:	 400f94c0             	sete	al
;;   20:	 4883c410             	add	rsp, 0x10
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
