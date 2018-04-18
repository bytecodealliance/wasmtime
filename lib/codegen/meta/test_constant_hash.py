from __future__ import absolute_import
import doctest
import constant_hash


def load_tests(loader, tests, ignore):
    tests.addTests(doctest.DocTestSuite(constant_hash))
    return tests
