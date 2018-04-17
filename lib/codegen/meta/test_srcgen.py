from __future__ import absolute_import
import doctest
import srcgen


def load_tests(loader, tests, ignore):
    tests.addTests(doctest.DocTestSuite(srcgen))
    return tests
