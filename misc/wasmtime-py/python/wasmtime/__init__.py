from .lib_wasmtime import imported_modules, instantiate
import sys
import os.path

from importlib import import_module
from importlib.abc import Loader, MetaPathFinder
from importlib.util import spec_from_file_location

# Mostly copied from
# https://stackoverflow.com/questions/43571737/how-to-implement-an-import-hook-that-can-modify-the-source-code-on-the-fly-using
class MyMetaFinder(MetaPathFinder):
    def find_spec(self, fullname, path, target=None):
        if path is None or path == "":
            path = [os.getcwd()] # top level import --
            path.extend(sys.path)
        if "." in fullname:
            *parents, name = fullname.split(".")
        else:
            name = fullname
        for entry in path:
            filename = os.path.join(entry, name + ".wasm")
            if not os.path.exists(filename):
                continue

            return spec_from_file_location(fullname, filename, loader=MyLoader(filename))
        return None

class MyLoader(Loader):
    def __init__(self, filename):
        self.filename = filename

    def create_module(self, spec):
        return None # use default module creation semantics

    def exec_module(self, module):
        with open(self.filename, "rb") as f:
            data = f.read()

        imports = {}
        for module_name, fields in imported_modules(data).items():
            imports[module_name] = {}
            imported_module = import_module(module_name)
            for field_name in fields:
                imports[module_name][field_name] = imported_module.__dict__[field_name]

        res = instantiate(data, imports)
        module.__dict__.update(res.instance.exports)

sys.meta_path.insert(0, MyMetaFinder())
