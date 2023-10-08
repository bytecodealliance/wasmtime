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
;;   11:	 4c8b5c2408           	mov	r11, qword ptr [rsp + 8]
;;   16:	 4153                 	push	r11
;;   18:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   1c:	 498b5b50             	mov	rbx, qword ptr [r11 + 0x50]
;;   20:	 4883ec08             	sub	rsp, 8
;;   24:	 4c89f7               	mov	rdi, r14
;;   27:	 be00000000           	mov	esi, 0
;;   2c:	 ba0a000000           	mov	edx, 0xa
;;   31:	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;   36:	 ffd3                 	call	rbx
;;   38:	 4883c410             	add	rsp, 0x10
;;   3c:	 4883c410             	add	rsp, 0x10
;;   40:	 5d                   	pop	rbp
;;   41:	 c3                   	ret	
