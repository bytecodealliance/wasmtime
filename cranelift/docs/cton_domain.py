# -*- coding: utf-8 -*-
#
# Sphinx domain for documenting compiler intermediate languages.
#
# This defines a 'cton' Sphinx domain with the following directives and roles:
#
# .. cton::type:: type
#     Document an IR type.
# .. cton:inst:: v1, v2 = inst op1, op2
#     Document an IR instruction.
#

import re

from docutils import nodes
from docutils.parsers.rst import directives

from sphinx import addnodes
from sphinx.directives import ObjectDescription
from sphinx.domains import Domain, ObjType
from sphinx.locale import l_, _
from sphinx.roles import XRefRole
from sphinx.util.docfields import Field, GroupedField, TypedField
from sphinx.util.nodes import make_refnode

import sphinx.ext.autodoc

class CtonObject(ObjectDescription):
    """
    Any kind of Cretonne IL object.

    This is a shared base class for the different kinds of indexable objects
    in the Cretonne IL reference.
    """
    option_spec = {
        'noindex': directives.flag,
        'module': directives.unchanged,
        'annotation': directives.unchanged,
    }

    def add_target_and_index(self, name, sig, signode):
        """
        Add ``name`` the the index.

        :param name: The object name returned by :func:`handle_signature`.
        :param sig: The signature text.
        :param signode: The output node.
        """
        targetname = self.objtype + '-' + name
        if targetname not in self.state.document.ids:
            signode['names'].append(targetname)
            signode['ids'].append(targetname)
            signode['first'] = (not self.names)
            self.state.document.note_explicit_target(signode)
            inv = self.env.domaindata['cton']['objects']
            if name in inv:
                self.state_machine.reporter.warning(
                    'duplicate Cretonne object description of %s, ' % name +
                    'other instance in ' + self.env.doc2path(inv[name][0]),
                    line=self.lineno)
            inv[name] = (self.env.docname, self.objtype)

        indextext = self.get_index_text(name)
        if indextext:
            self.indexnode['entries'].append(('single', indextext,
                                              targetname, ''))

# Type variables are indicated as %T.
typevar = re.compile('(\%[A-Z])')

def parse_type(name, signode):
    """
    Parse a type with embedded type vars and append to signode.
    
    Return a a string that can be compiled into a regular expression matching
    the type.
    """

    re_str = ''

    for part in typevar.split(name):
        if part == '':
            continue
        if len(part) == 2 and part[0] == '%':
            # This is a type parameter. Don't display the %, use emphasis
            # instead.
            part = part[1]
            signode += nodes.emphasis(part, part)
            re_str += r'\w+'
        else:
            signode += addnodes.desc_name(part, part)
            re_str += re.escape(part)
    return re_str

class CtonType(CtonObject):
    """A Cretonne IL type description."""

    def handle_signature(self, sig, signode):
        """
        Parse type signature in ``sig`` and append description to signode.

        Return a global object name for ``add_target_and_index``.
        """

        name = sig.strip()
        re_str = parse_type(name, signode)
        return name

    def get_index_text(self, name):
        return name + ' (IL type)'

sep_equal = re.compile('\s*=\s*')
sep_comma = re.compile('\s*,\s*')

def parse_params(s, signode):
    for i,p in enumerate(sep_comma.split(s)):
        if i != 0:
            signode += nodes.Text(', ')
        signode += nodes.emphasis(p, p)

class CtonInst(CtonObject):
    """A Cretonne IL instruction."""

    doc_field_types = [
        TypedField('argument', label=l_('Arguments'),
                   names=('in', 'arg'),
                   typerolename='type', typenames=('type',)),
        TypedField('result', label=l_('Results'),
                   names=('out', 'result'),
                   typerolename='type', typenames=('type',)),
        GroupedField('flag', names=('flag',), label=l_('Flags')),
        Field('resulttype', label=l_('Result type'), has_arg=False,
              names=('rtype',)),
    ]

    def handle_signature(self, sig, signode):
        # Look for signatures like
        #
        #   v1, v2 = foo op1, op2
        #   v1 = foo
        #   foo op1

        parts = re.split(sep_equal, sig, 1)
        if len(parts) == 2:
            # Outgoing parameters.
            parse_params(parts[0], signode)
            signode += nodes.Text(' = ')
            name = parts[1]
        else:
            name = parts[0]

        # Parse 'name arg, arg'
        parts = name.split(None, 1)
        name = parts[0]
        signode += addnodes.desc_name(name, name)

        if len(parts) == 2:
            # Incoming parameters.
            signode += nodes.Text(' ')
            parse_params(parts[1], signode)

        return name

    def get_index_text(self, name):
        return name

class CretonneDomain(Domain):
    """Cretonne domain for intermediate language objects."""
    name = 'cton'
    label = 'Cretonne'

    object_types = {
        'type' : ObjType(l_('type'), 'type'),
        'inst' : ObjType(l_('instruction'), 'inst')
    }

    directives = {
        'type' : CtonType,
        'inst' : CtonInst,
    }

    roles = {
        'type' : XRefRole(),
        'inst' : XRefRole(),
    }

    initial_data = {
        'objects': {},  # fullname -> docname, objtype
    }

    def clear_doc(self, docname):
        for fullname, (fn, _l) in list(self.data['objects'].items()):
            if fn == docname:
                del self.data['objects'][fullname]

    def merge_domaindata(self, docnames, otherdata):
        for fullname, (fn, objtype) in otherdata['objects'].items():
            if fn in docnames:
                self.data['objects'][fullname] = (fn, objtype)

    def resolve_xref(self, env, fromdocname, builder, typ, target, node,
                     contnode):
        objects = self.data['objects']
        if target not in objects:
            return None
        obj = objects[target]
        return make_refnode(builder, fromdocname, obj[0],
                            obj[1] + '-' + target, contnode, target)

    def resolve_any_xref(self, env, fromdocname, builder, target,
                         node, contnode):
        objects = self.data['objects']
        if target not in objects:
            return []
        obj = objects[target]
        return [('cton:' + self.role_for_objtype(obj[1]),
                 make_refnode(builder, fromdocname, obj[0],
                              obj[1] + '-' + target, contnode, target))]


class TypeDocumenter(sphinx.ext.autodoc.Documenter):
    # Invoke with .. autoctontype::
    objtype = 'ctontype'
    # Convert into cton:type directives
    domain = 'cton'
    directivetype = 'type'

    @classmethod
    def can_document_member(cls, member, membername, isattr, parent):
        return False

    def resolve_name(self, modname, parents, path, base):
        return 'cretonne.types', [ base ]

    def add_content(self, more_content, no_docstring=False):
        super(TypeDocumenter, self).add_content(more_content, no_docstring)
        sourcename = self.get_sourcename()
        membytes = self.object.membytes
        if membytes:
            self.add_line(u':bytes: {}'.format(membytes), sourcename)
        else:
            self.add_line(u':bytes: Can\'t be stored in memory', sourcename)


def setup(app):
    app.add_domain(CretonneDomain)
    app.add_autodocumenter(TypeDocumenter)

    return { 'version' : '0.1' }
