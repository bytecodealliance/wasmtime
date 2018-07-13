" Vim syntax file
" Language:     Cranelift
" Maintainer:   Jakob Stoklund Olesen <stoklund@2pi.dk
" Last Change:  Jun 16, 2017

if version < 600
  syntax clear
elseif exists("b:current_syntax")
  finish
endif

" Disable spell checking even in comments.
" They tend to refer to weird stuff like assembler mnemonics anyway.
syn spell notoplevel

syn keyword clifHeader test isa set
syn keyword clifDecl function jump_table incoming_arg outgoing_arg spill_slot explicit_slot emergency_slot
syn keyword clifFilecheck check sameln nextln unordered not regex contained

syn match clifType  /\<\([bif]\d\+\(x\d\+\)\?\)\|[if]flags\>/
syn match clifEntity /\<\(v\|ss\|jt\|fn\|sig\)\d\+\>/
syn match clifLabel /\<ebb\d+\>/
syn match clifName /%\w\+\>/

syn match clifNumber /-\?\<[0-9_]\+\>/
syn match clifNumber /-\?\<0x[0-9a-fA-F_]\+\(\.[0-9a-fA-F_]*\)\?\(p[+-]\?\d\+\)\?\>/
syn match clifHexSeq /#\x\+\>/
syn match clifSourceLoc /@[0-9a-f]\+\>/

syn region clifCommentLine start=";" end="$" contains=clifFilecheck

hi def link clifHeader        Keyword
hi def link clifDecl          Keyword
hi def link clifType          Type
hi def link clifEntity        Identifier
hi def link clifLabel         Label
hi def link clifName          String
hi def link clifNumber        Number
hi def link clifHexSeq        Number
hi def link clifCommentLine   Comment
hi def link clifFilecheck     SpecialComment
hi def link clifSourceLoc     LineNr

let b:current_syntax = "clif"
