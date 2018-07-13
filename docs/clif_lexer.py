# -*- coding: utf-8 -*-
#
# Pygments lexer for Cranelift.
from __future__ import absolute_import

from pygments.lexer import RegexLexer, bygroups, words
from pygments.token import Comment, String, Keyword, Whitespace, Number, Name
from pygments.token import Operator, Punctuation, Text


def keywords(*args):
    return words(args, prefix=r'\b', suffix=r'\b')


class CraneliftLexer(RegexLexer):
    name = 'Cranelift'
    aliases = ['clif']
    filenames = ['*.clif']

    tokens = {
        'root': [
            # Test header lines.
            (r'^(test|isa|set)(?:( +)([-\w]+)' +
             r'(?:(=)(?:(\d+)|(yes|no|true|false|on|off)|(\w+)))?)*' +
             r'( *)$',
                bygroups(Keyword.Namespace, Whitespace, Name.Attribute,
                         Operator, Number.Integer, Keyword.Constant,
                         Name.Constant, Whitespace)),
            # Comments with filecheck or other test directive.
            (r'(; *)([a-z]+:)(.*?)$',
                bygroups(Comment.Single, Comment.Special, Comment.Single)),
            # Plain comments.
            (r';.*?$', Comment.Single),
            # Strings are prefixed by % or # with hex.
            (r'%\w+|#[0-9a-fA-F]*', String),
            # Numbers.
            (r'[-+]?0[xX][0-9a-fA-F_]+', Number.Hex),
            (r'[-+]?0[xX][0-9a-fA-F_]*\.[0-9a-fA-F_]*([pP]\d+)?', Number.Hex),
            (r'[-+]?([0-9_]+\.[0-9_]+([eE]\d+)?|s?NaN|Inf)', Number.Float),
            (r'[-+]?[0-9_]+', Number.Integer),
            # Known attributes.
            (keywords('uext', 'sext'), Name.Attribute),
            # Well known value types.
            (r'\b(b\d+|i\d+|f32|f64)(x\d+)?\b', Keyword.Type),
            # v<nn> = value
            # ss<nn> = stack slot
            # jt<nn> = jump table
            (r'(v|ss|gv|jt|fn|sig|heap)\d+', Name.Variable),
            # ebb<nn> = extended basic block
            (r'(ebb)\d+', Name.Label),
            # Match instruction names in context.
            (r'(=)( *)([a-z]\w*)',
                bygroups(Operator, Whitespace, Name.Function)),
            (r'^( *)([a-z]\w*\b)(?! *[,=])',
                bygroups(Whitespace, Name.Function)),
            # Other names: results and arguments
            (r'[a-z]\w*', Name),
            (r'->|=|:', Operator),
            (r'[{}(),.]', Punctuation),
            (r'[ \t]+', Text),
        ],
    }


def setup(app):
    """Setup Sphinx extension."""
    app.add_lexer('clif', CraneliftLexer())

    return {'version': '0.1'}
