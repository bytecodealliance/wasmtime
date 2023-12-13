;;! target = "x86_64"

(module
    (func (result i32)
        i64.const 1
        i32.wrap_i64
        block
        end
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 89c0                 	mov	eax, eax
;;   15:	 4883ec04             	sub	rsp, 4
;;   19:	 890424               	mov	dword ptr [rsp], eax
;;   1c:	 8b0424               	mov	eax, dword ptr [rsp]
;;   1f:	 4883c404             	add	rsp, 4
;;   23:	 4883c408             	add	rsp, 8
;;   27:	 5d                   	pop	rbp
;;   28:	 c3                   	ret	
