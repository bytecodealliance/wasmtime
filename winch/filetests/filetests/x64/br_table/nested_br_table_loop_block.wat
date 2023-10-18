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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 b902000000           	mov	ecx, 2
;;   19:	 39c1                 	cmp	ecx, eax
;;   1b:	 0f42c1               	cmovb	eax, ecx
;;   1e:	 4c8d1d0a000000       	lea	r11, [rip + 0xa]
;;   25:	 49630c83             	movsxd	rcx, dword ptr [r11 + rax*4]
;;   29:	 4901cb               	add	r11, rcx
;;   2c:	 41ffe3               	jmp	r11
;;   2f:	 e1ff                 	loope	0x30
