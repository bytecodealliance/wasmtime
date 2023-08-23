;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-br_if-last") (param i32) (result i32)
    (block (result i32)
      (br_if 0
        (i32.const 2)
        (if (result i32) (local.get 0)
          (then (call $dummy) (i32.const 1))
          (else (call $dummy) (i32.const 0))
        )
      )
      (return (i32.const 3))
    )
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
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 85c0                 	test	eax, eax
;;   17:	 0f840f000000         	je	0x2c
;;   1d:	 e800000000           	call	0x22
;;   22:	 b801000000           	mov	eax, 1
;;   27:	 e90a000000           	jmp	0x36
;;   2c:	 e800000000           	call	0x31
;;   31:	 b800000000           	mov	eax, 0
;;   36:	 50                   	push	rax
;;   37:	 59                   	pop	rcx
;;   38:	 b802000000           	mov	eax, 2
;;   3d:	 85c9                 	test	ecx, ecx
;;   3f:	 0f850a000000         	jne	0x4f
;;   45:	 50                   	push	rax
;;   46:	 b803000000           	mov	eax, 3
;;   4b:	 4883c408             	add	rsp, 8
;;   4f:	 4883c410             	add	rsp, 0x10
;;   53:	 5d                   	pop	rbp
;;   54:	 c3                   	ret	
