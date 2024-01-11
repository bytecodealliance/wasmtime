;;! target = "x86_64"

(module
  (table $t1 0 funcref)

  (func (export "grow-by-10") (param $r funcref) (result i32)
    (table.grow $t1 (local.get $r) (i32.const 10))
  )
)


;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;    d:	 4c893424             	mov	qword ptr [rsp], r14
;;   11:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   15:	 498b5b50             	mov	rbx, qword ptr [r11 + 0x50]
;;   19:	 4156                 	push	r14
;;   1b:	 4c8b5c2410           	mov	r11, qword ptr [rsp + 0x10]
;;   20:	 4153                 	push	r11
;;   22:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;   27:	 be00000000           	mov	esi, 0
;;   2c:	 ba0a000000           	mov	edx, 0xa
;;   31:	 488b0c24             	mov	rcx, qword ptr [rsp]
;;   35:	 ffd3                 	call	rbx
;;   37:	 4883c410             	add	rsp, 0x10
;;   3b:	 4883c410             	add	rsp, 0x10
;;   3f:	 5d                   	pop	rbp
;;   40:	 c3                   	ret	
