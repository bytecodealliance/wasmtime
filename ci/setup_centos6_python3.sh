#!/bin/bash
set -e

VERSION=${1:-3.7.3}

# Python 3.6 stands in our way -- nuking it
yum erase -y rh-python36
rm -rf /opt/rh/rh-python36

yum install -y gcc bzip2-devel libffi-devel zlib-devel

cd /usr/src/

# pip3.7 needs new openssl
curl -O -L https://github.com/openssl/openssl/archive/OpenSSL_1_1_1c.tar.gz
tar -zxvf OpenSSL_1_1_1c.tar.gz
cd openssl-OpenSSL_1_1_1c
./Configure shared zlib linux-x86_64
make -sj4
make install
cd ..
rm -rf openssl-OpenSSL_1_1_1c

# Fixing libssl.so.1.1: cannot open shared object file
echo "/usr/local/lib64" >> /etc/ld.so.conf && ldconfig

curl -O -L https://www.python.org/ftp/python/${VERSION}/Python-${VERSION}.tgz
tar xzf Python-${VERSION}.tgz
cd Python-${VERSION}
./configure
make -sj4
make install
cd ..
rm -rf Python-${VERSION}
