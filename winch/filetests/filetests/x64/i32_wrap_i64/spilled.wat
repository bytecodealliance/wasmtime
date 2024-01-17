;;! target = "x86_64"

(module
    (func (result i32)
        i64.const 1
        i32.wrap_i64
        block
        end
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c001000000       	mov	rax, 1
;;      	 89c0                 	mov	eax, eax
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 8b0424               	mov	eax, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
