#!/bin/bash

set -e

if [ "$#" -ne 1 ]; then
    version="2020.4.287"
else
    version="$1"
fi

scriptdir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# Retrieve OpenVINO checksum.
curl -sSL https://apt.repos.intel.com/openvino/2020/GPG-PUB-KEY-INTEL-OPENVINO-2020 > $scriptdir/GPG-PUB-KEY-INEL-OPENVINO-2020
echo "5f5cff8a2d26ba7de91942bd0540fa4d $scriptdir/GPG-PUB-KEY-INTEL-OPENVINO-2020" > $scriptdir/CHECKSUM
md5sum --check $scriptdir/CHECKSUM

# Add OpenVINO repository (deb).
sudo apt-key add $scriptdir/GPG-PUB-KEY-INTEL-OPENVINO-2020
echo "deb https://apt.repos.intel.com/openvino/2020 all main" | sudo tee /etc/apt/sources.list.d/intel-openvino-2020.list
sudo apt update

# Install OpenVINO package.
sudo apt install -y intel-openvino-runtime-ubuntu18-$version
