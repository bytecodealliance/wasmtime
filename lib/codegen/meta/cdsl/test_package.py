from __future__ import absolute_import
import doctest
import cdsl


def load_tests(loader, tests, ignore):
    tests.addTests(doctest.DocTestSuite(cdsl))
    return tests
