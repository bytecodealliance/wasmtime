from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(name='wasmtime',
      version="0.0.1",
      classifiers=[
            "Development Status :: 1 - Planning",
            "Intended Audience :: Developers",
            "Programming Language :: Python",
            "Programming Language :: Rust",
            "Operating System :: POSIX",
            "Operating System :: MacOS :: MacOS X",
            "Operating System :: Microsoft :: Windows",
      ],
      packages=['wasmtime'],
      package_dir={'wasmtime': 'python/wasmtime'},
      rust_extensions=[RustExtension('wasmtime.lib_wasmtime', 'Cargo.toml',  binding=Binding.PyO3)],
      zip_safe=False)
