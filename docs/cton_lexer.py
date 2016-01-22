# -*- coding: utf-8 -*-
#
# Pygments lexer for Cretonne.

from pygments.lexer import RegexLexer, bygroups
from pygments.token import *

class CretonneLexer(RegexLexer):
    name = 'Cretonne'
    aliases = ['cton']
    filenames = ['*.cton']

    tokens = {
        'root': [
            (r';.*?$', Comment.Single),
            (r'\b(function|entry)\b', Keyword),
            (r'\b(align)\b', Name.Attribute),
            # Well known value types.
            (r'\b(bool|i\d+|f32|f64)(x\d+)?\b', Keyword.Type),
            (r'\d+', Number.Integer),
            (r'0[xX][0-9a-fA-F]+', Number.Hex),
            # v<nn> = value
            # ss<nn> = stack slot
            (r'(v|ss)\d+', Name.Variable),
            # ebb<nn> = extended basic block
            (r'(ebb)\d+', Name.Label),
            (r'(=)( *)([a-z]\w*)', bygroups(Operator, Whitespace, Name.Function)),
            (r'^( +)([a-z]\w*\b)(?! *[,=])', bygroups(Whitespace, Name.Function)),
            (r'[a-z]\w*', Name),
            (r'->|=|:', Operator),
            (r'[{}(),.]', Punctuation),
            (r'[ \t]+', Text),
        ]
    }

def setup(app):
    """Setup Sphinx extension."""
    app.add_lexer('cton', CretonneLexer())

    return { 'version' : '0.1' }
