;;! target = "x86_64"
;;! flags = ["has_popcnt", "has_sse42"]

(module
    (func (result i64)
      i64.const 3
      i64.popcnt
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c003000000       	mov	rax, 3
;;   13:	 f3480fb8c0           	popcnt	rax, rax
;;   18:	 4883c408             	add	rsp, 8
;;   1c:	 5d                   	pop	rbp
;;   1d:	 c3                   	ret	
