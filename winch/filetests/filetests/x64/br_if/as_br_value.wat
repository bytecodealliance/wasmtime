;;! target = "x86_64"
(module
  (func (export "as-br-value") (result i32)
    (block (result i32) (br 0 (br_if 0 (i32.const 1) (i32.const 2))))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b902000000           	mov	ecx, 2
;;   11:	 b801000000           	mov	eax, 1
;;   16:	 85c9                 	test	ecx, ecx
;;   18:	 0f8500000000         	jne	0x1e
;;   1e:	 4883c408             	add	rsp, 8
;;   22:	 5d                   	pop	rbp
;;   23:	 c3                   	ret	
