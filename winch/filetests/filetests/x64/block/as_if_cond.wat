;;! target = "x86_64"

(module
  (func $dummy)
  (func (export "as-if-condition")
   (block (result i32) (i32.const 1)) (if (then (call $dummy)))
  )
)
  
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 85c0                 	test	eax, eax
;;   15:	 0f840d000000         	je	0x28
;;   1b:	 4883ec08             	sub	rsp, 8
;;   1f:	 e800000000           	call	0x24
;;   24:	 4883c408             	add	rsp, 8
;;   28:	 4883c408             	add	rsp, 8
;;   2c:	 5d                   	pop	rbp
;;   2d:	 c3                   	ret	
