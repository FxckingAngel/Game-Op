from setuptools import setup
from Cython.Build import cythonize

setup(
    ext_modules=cythonize([
        "asset_key_resolver_bin.py",
        "bundle_optimizer_bin.py"
    ], compiler_directives={'language_level': "3"})
)
