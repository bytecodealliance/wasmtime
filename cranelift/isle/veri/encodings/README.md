Encodings Generation
====================

This directory contains some templatized SMT-LIBv2 code that encodes some operations we need in the verifier.
The way this works is that the `*.smt2` file contains something close to usable SMT-LIBv2 code that can be developed independently, modulo some Rust template expressions like `{this}`.
Then, `convert.py` translates these files into Rust code that can *generate* these S-expressions for the `easy-smt` library.

Using this code is not yet a fully automated process.
If you make changes to an encoding, do this:

* Run `python3 convert.py rev8.smt2 | pbcopy` or similar to copy the new Rust code.
* Find the appropriate Rust function in `veri_engine/src/solver/encoded_ops`.
  Delete the old stanza of generated code and replace it with your new code.
* Be careful to preserve the extraction and padding expressions at the top and bottom of the function, respectively.
  Adjust them to match the variable names from the generated code, if necessary.
