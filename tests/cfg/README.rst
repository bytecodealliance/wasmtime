CFG tests
============

This directory contains test cases for the Cretonne cfg printer.

Each test case consists of a `foo.cton` input file annotated with its expected connections.
Annotations are comments of the form: `ebbx:insty -> ebbz` where ebbx is connected to ebbz via
a branch or jump instruction at line y. Instructions are labeled by line number starting from zero: `inst0` .. `instn`.


Each input file is run through the `cton-util print-cfg` command and the
output is compared against the specially formatted comments to ensure that
expected connections exist. This scheme allows for changes to graph style
without the need to update tests.
