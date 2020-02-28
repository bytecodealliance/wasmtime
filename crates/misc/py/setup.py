from setuptools import setup
from setuptools_rust import Binding, RustExtension


def no_tag_default_to_dev(version):
    if version.exact:
        return version.format_with("{tag}")
    return "0.0.1"


setup(name='wasmtime',
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
      use_scm_version = {
          "root": "../../..",
          "relative_to": __file__,
          "version_scheme": no_tag_default_to_dev,
          "local_scheme": lambda _: "",
      },
      setup_requires=['setuptools_scm'],
      rust_extensions=[RustExtension('wasmtime.lib_wasmtime', 'Cargo.toml',  binding=Binding.PyO3)],
      zip_safe=False)
