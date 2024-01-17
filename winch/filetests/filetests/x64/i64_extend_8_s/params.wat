;;! target = "x86_64"

(module
    (func (param i64) (result i64)
        (local.get 0)
        (i64.extend8_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 480fbec0             	movsx	rax, al
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
