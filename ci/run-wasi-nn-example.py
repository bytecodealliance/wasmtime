# The following script demonstrates how to execute a machine learning inference
# using the wasi-nn module optionally compiled into Wasmtime. Calling it will
# download the necessary model and tensor files stored separately in $FIXTURE
# into $TMP_DIR (optionally pass a directory with existing files as the first
# argument to re-try the script). Then, it will compile and run several examples
# in the Wasmtime CLI.

from pathlib import Path
import tempfile
import subprocess
import sys
import urllib.request
import os
import shutil

MODEL_URL = 'https://github.com/onnx/models/raw/5faef4c33eba0395177850e1e31c4a6a9e634c82/vision/classification/mobilenet/model/mobilenetv2-12.onnx'
IMG_URL = 'https://github.com/microsoft/Windows-Machine-Learning/blob/master/SharedContent/media/kitten_224.png?raw=true'

WASMTIME_DIR = Path(__file__).resolve().parent.parent
TMPD = tempfile.TemporaryDirectory()
TMP_DIR = Path(TMPD.name)
# TMP_DIR = Path(r"E:\temp\wasi-nn")


def build():
    # Build Wasmtime with wasi-nn enabled; we attempt this first to avoid extra work if the build fails.
    subprocess.call(
        ['cargo', 'build', '-p', 'wasmtime-cli', '--features', 'wasi-nn'], cwd=WASMTIME_DIR)
    # Now build an example that uses the wasi-nn API. Run the example in Wasmtime
    subprocess.call(['cargo', 'build', '--release', '--target=wasm32-wasi'],
                    cwd=WASMTIME_DIR/'crates'/'wasi-nn'/'examples'/'classification-example-winml')


def download_fixture():
    (TMP_DIR/'mobilenet').mkdir(exist_ok=True)
    urllib.request.urlretrieve(MODEL_URL, (TMP_DIR/'mobilenet'/'mobilenet.onnx').resolve())
    urllib.request.urlretrieve(IMG_URL, (TMP_DIR/'kitten.png').resolve())


def run_tests():
    wasm_file_name = 'wasi-nn-example-winml.wasm'
    classification_example_winml_dir = WASMTIME_DIR/'crates' / \
        'wasi-nn'/'examples'/'classification-example-winml'
    shutil.copy(classification_example_winml_dir/'target' /
                'wasm32-wasi'/'release'/wasm_file_name, TMP_DIR)
    subprocess.call(['cargo', 'run', '--', 'run', '--dir', 'fixture::'+str(TMP_DIR.resolve()), '-S', 'nn,nn-graph=onnx::'+str((TMP_DIR/'mobilenet').resolve()),
                     str((TMP_DIR/wasm_file_name).resolve())], cwd=WASMTIME_DIR)


def main():
    build()
    download_fixture()
    run_tests()

    return 0


if __name__ == '__main__':
    sys.exit(main())
