#!/bin/bash

set -e

# Retrieve OpenVINO checksum.
wget https://apt.repos.intel.com/openvino/2020/GPG-PUB-KEY-INTEL-OPENVINO-2020
echo '5f5cff8a2d26ba7de91942bd0540fa4d  GPG-PUB-KEY-INTEL-OPENVINO-2020' > CHECKSUM
md5sum --check CHECKSUM

# Add OpenVINO repository (deb).
sudo apt-key add GPG-PUB-KEY-INTEL-OPENVINO-2020
echo "deb https://apt.repos.intel.com/openvino/2020 all main" | sudo tee /etc/apt/sources.list.d/intel-openvino-2020.list
sudo apt update

# Install OpenVINO package.
sudo apt install -y intel-openvino-runtime-ubuntu18-$1
