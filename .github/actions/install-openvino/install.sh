#!/bin/bash

set -e

# Determine the OpenVINO version to install from the first parameter. Also, split out the parts of
# this version; `${version_parts[0]}` should contain the year. E.g.:
#  version=2021.4.752
#  version_year=2021
if [ "$#" -ne 1 ]; then
    version="2021.4.752"
else
    version="$1"
fi
IFS='.' read -ra version_parts <<< "$version"
version_year="${version_parts[0]}"

# Determine the OS name and version (Linux-specific for now). E.g.:
#  os_name=ubuntu
#  os_version=20.04
#  os_version_year=20
eval $(source /etc/os-release; echo os_name="$ID"; echo os_version="$VERSION_ID";)
IFS='.' read -ra os_version_parts <<< "$os_version"
os_version_year="${os_version_parts[0]}"

# Determine the directory of this script. E.g.:
#  script_dir=/some/directory
scriptdir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# Retrieve the OpenVINO checksum.
curl -sSL https://apt.repos.intel.com/openvino/$version_year/GPG-PUB-KEY-INTEL-OPENVINO-$version_year > $scriptdir/GPG-PUB-KEY-INTEL-OPENVINO-$version_year
echo "5f5cff8a2d26ba7de91942bd0540fa4d $scriptdir/GPG-PUB-KEY-INTEL-OPENVINO-$version_year" > $scriptdir/CHECKSUM
md5sum --check $scriptdir/CHECKSUM

# Add the OpenVINO repository (DEB-specific for now).
sudo apt-key add $scriptdir/GPG-PUB-KEY-INTEL-OPENVINO-$version_year
echo "deb https://apt.repos.intel.com/openvino/$version_year all main" | sudo tee /etc/apt/sources.list.d/intel-openvino-$version_year.list
sudo apt update

# Install the OpenVINO package.
sudo apt install -y intel-openvino-runtime-$os_name$os_version_year-$version
