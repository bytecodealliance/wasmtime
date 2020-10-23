# install-openvino

A GitHub action to install OpenVINO from a package repository. This is only necessary for `wasi-nn` support but there
are enough steps here to package the functionality separately and avoid cluttering the CI.

Future improvements:
 - make this installer work for different OS/distributions (e.g. https://docs.openvinotoolkit.org/latest/openvino_docs_install_guides_installing_openvino_windows.html)
 - it would be nice to output the install directory (i.e. `/opt/intel/openvino`)
