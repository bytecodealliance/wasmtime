;;! target = "x86_64"
;;! flags = ["has_popcnt", "has_sse42"]

(module
    (func (result i32)
      i32.const 3
      i32.popcnt
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b803000000           	mov	eax, 3
;;      	 f30fb8c0             	popcnt	eax, eax
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
