;;! target = "x86_64"
;;! flags = ["has_lzcnt"]

(module
    (func (result i32)
        (i32.const 1)
        (i32.clz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 f30fbdc0             	lzcnt	eax, eax
;;   15:	 4883c408             	add	rsp, 8
;;   19:	 5d                   	pop	rbp
;;   1a:	 c3                   	ret	
