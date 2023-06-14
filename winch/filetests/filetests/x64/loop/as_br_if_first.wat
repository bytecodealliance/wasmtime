;;! target = "x86_64"
(module
  (func (export "as-br-if-first") (result i32)
    (block (result i32) (br_if 0 (loop (result i32) (i32.const 1)) (i32.const 2)))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b902000000           	mov	ecx, 2
;;   11:	 48c7c001000000       	mov	rax, 1
;;   18:	 85c9                 	test	ecx, ecx
;;   1a:	 0f8500000000         	jne	0x20
;;   20:	 4883c408             	add	rsp, 8
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
