import sys
import re
import sexpdata

DECL = "(declare-fun "
ASSERTION = "(assert "
PATTERN = r'\{(.*?)\}'
SMT_FUNCS = [
    'bvand',
    'bvor',
    'bvshl',
    'bvlshr',
    'bvashr',
]


def sexpr_to_rs(sexpr):
    """Generate Rust code to generate an S-expression.

    Convert a parsed S-expression to Rust code (as a string) that
    generates the same thing. The generated code makes calls to a
    `solver` context struct.

    Symbols containing Rust formatting delimiters, like `{this}`, are
    treated specially. Instead of generating liter symbols, we generate
    references to corresponding Rust variables that should hold those
    symbols.
    """
    if isinstance(sexpr, sexpdata.Symbol):
        sym = sexpr.value()

        if sym == '{x}':  # The special input expression.
            return 'x'
        elif sym.endswith('_{id}'):  # Tagged "local" variables.
            base, ext = sym.rsplit('_', 1)
            return base
        elif sym == '_':
            return 'solver.smt.atoms().und'

        # General case: construct an atom.
        assert '{' not in sym  # Guard against unhandled templates.
        return f'solver.smt.atom("{sym}")'

    elif isinstance(sexpr, list):
        # Special cases for common SMT functions.
        if sexpr:
            hd, tl = sexpr[0], sexpr[1:]
            tl_rs = ", ".join(sexpr_to_rs(v) for v in tl)
            if hd == sexpdata.Symbol('='):
                return f'solver.smt.eq({tl_rs})'
            elif isinstance(hd, sexpdata.Symbol) and hd.value() in SMT_FUNCS:
                return f'solver.smt.{hd.value()}({tl_rs})'

        # General case: construct a normal list.
        guts = ", ".join(sexpr_to_rs(v) for v in sexpr)
        return f'solver.smt.list(vec![{guts}])'

    elif isinstance(sexpr, int):
        return f'solver.smt.numeral({sexpr})'
    else:
        assert False


def parse_decl(line):
    """Parse a `declare-fun` line.

    The line must look like `(declare-fun <name> () <type>)`. Return a
    tuple consisting of:

    * The Rust variable name we should use for the declared SMT variable.
    * A Rust expression for the SMT variable's name (a string).
    * A Rust expression for the type (an SExpr).
    """
    # Parse the S-expression.
    exp = sexpdata.loads(line)
    assert exp[0].value() == 'declare-fun'
    _, name, args, ret = exp
    name = name.value()

    # Rust code to format the variable name. Format with Rust variables
    # matching the variables in the format string.
    matches = re.findall(PATTERN, name)
    if matches:
        named_params = ', '.join([f'{x} = {x}' for x in matches])
        name_rs = f'format!(\"{name}\", {named_params})'
    else:
        name_rs = name  

    # Strip off the '_{id}' to get the Rust variable name.
    assert name.endswith('_{id}')
    name_var, _ = name.rsplit('_', 1)

    return name_var, name_rs, sexpr_to_rs(ret)


def parse_assertion(line):
    """Parse an `assert` line.

    The line must look like `(assert <assertion>)`. Return Rust code to
    generate an equivalent S-expression for the underlying assertion.
    """
    # Parse the S-expression.
    exp = sexpdata.loads(line)
    assert exp[0].value() == 'assert'
    _, asst = exp

    return sexpr_to_rs(asst)


def main():
    filename = sys.argv[1]

    with open(filename, 'r') as f:
        for line in f:
            line = line.strip()

            # Convert comments.
            if line.startswith(';'):
                print(f'//{line[1:]}')
                continue

            # Convert declarations.
            if line.startswith(DECL):
                var, name, ty = parse_decl(line)
                print(f'let {var} = solver.declare({name}, {ty});')
                continue

            # Convert assertions.
            if line.startswith(ASSERTION):
                a = parse_assertion(line)
                print(f'solver.assume({a});')
                continue


if __name__ == '__main__':
    main()
