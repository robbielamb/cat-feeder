# Enironment variables for building cross compiling to a raspberry pi.
# This file is sourced by the build scripts `build-armv7.sh` and `build-arm.sh`
# It may also be sourced right into the current working environment

SYSROOT=$(pwd)/arm-udev

export PKG_CONFIG_DIR=
export PKG_CONFIG_PATH=${SYSROOT}/usr/lib/arm-linux-gnueabihf/pkgconfig
export PKG_CONFIG_LIBDIR=${SYSROOT}/usr/lib/pkgconfig:${SYSROOT}/usr/share/pkgconfig
export PKG_CONFIG_SYSROOT_DIR=${SYSROOT}
export PKG_CONFIG_ALLOW_CROSS=1

MMAL_DIR=$(pwd)/rasppi-vc
export MMAL_INCLUDE_DIR=$MMAL_DIR/include
export MMAL_LIB_DIR=$MMAL_DIR/lib
