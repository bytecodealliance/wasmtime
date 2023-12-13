;;! target = "x86_64"

(module
    (func (result i32)
        i32.const 1
        f32.reinterpret_i32
        drop
        i32.const 1
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 660f6ec0             	movd	xmm0, eax
;;   15:	 b801000000           	mov	eax, 1
;;   1a:	 4883c408             	add	rsp, 8
;;   1e:	 5d                   	pop	rbp
;;   1f:	 c3                   	ret	
