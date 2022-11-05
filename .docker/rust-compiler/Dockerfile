FROM ubuntu:22.04
RUN apt-get update
RUN apt-get install -y wget build-essential gcc-aarch64-linux-gnu git
RUN mkdir -p /etc/arm-dependencies
RUN cd /etc/arm-dependencies && wget -O openssl-3.0.7.tar.gz https://www.openssl.org/source/openssl-3.0.7.tar.gz
RUN cd /etc/arm-dependencies && tar xvzf openssl-3.0.7.tar.gz
RUN cd /etc/arm-dependencies/openssl-3.0.7 && MACHINE=aarch64 CC=aarch64-linux-gnu-gcc ARCH=aarch64 ./Configure linux-generic64 shared
RUN cd /etc/arm-dependencies/openssl-3.0.7 && MACHINE=aarch64 CC=aarch64-linux-gnu-gcc ARCH=aarch64 make
