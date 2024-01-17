;;! target = "x86_64"

(module
    (func (param i64) (result i32)
        (local.get 0)
        (i64.eqz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 4883f800             	cmp	rax, 0
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
