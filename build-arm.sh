#!/bin/bash

SYSROOT=$(pwd)/arm-udev

export PKG_CONFIG_DIR=
export PKG_CONFIG_PATH=${SYSROOT}/usr/lib/arm-linux-gnueabihf/pkgconfig
export PKG_CONFIG_LIBDIR=${SYSROOT}/usr/lib/pkgconfig:${SYSROOT}/usr/share/pkgconfig
export PKG_CONFIG_SYSROOT_DIR=${SYSROOT}
export PKG_CONFIG_ALLOW_CROSS=1

MMAL_DIR=$(pwd)/rasppi-vc
export MMAL_INCLUDE_DIR=$MMAL_DIR/include
export MMAL_LIB_DIR=$MMAL_DIR/lib

cargo build --target arm-unknown-linux-gnueabihf
