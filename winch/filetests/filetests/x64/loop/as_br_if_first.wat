;;! target = "x86_64"
(module
  (func (export "as-br-if-first") (result i32)
    (block (result i32) (br_if 0 (loop (result i32) (i32.const 1)) (i32.const 2)))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b902000000           	mov	ecx, 2
;;      	 b801000000           	mov	eax, 1
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8500000000         	jne	0x1e
;;   1e:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
