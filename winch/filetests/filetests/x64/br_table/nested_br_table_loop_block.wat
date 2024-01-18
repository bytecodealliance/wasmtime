;;! target = "x86_64"
(module
  (func (export "nested-br_table-loop-block") (param i32) (result i32)
    (local.set 0
      (loop (result i32)
        (block
          (br_table 1 0 0 (local.get 0))
        )
        (i32.const 0)
      )
    )
    (loop (result i32)
      (block
        (br_table 0 1 1 (local.get 0))
      )
      (i32.const 3)
    )
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8777000000         	ja	0x8f
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 b902000000           	mov	ecx, 2
;;      	 39c1                 	cmp	ecx, eax
;;      	 0f42c1               	cmovb	eax, ecx
;;      	 4c8d1d0a000000       	lea	r11, [rip + 0xa]
;;      	 49630c83             	movsxd	rcx, dword ptr [r11 + rax*4]
;;      	 4901cb               	add	r11, rcx
;;      	 41ffe3               	jmp	r11
;;   3f:	 e1ff                 	loope	0x40
