//use std::env;
////use std::path::Path;
//
//fn main() {
//    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
//
//    println!("cargo:rustc-env=PKG_CONFIG_PATH={}/arm-udev/usr/lib/arm-linux-gnueabihf/pkgconfig", manifest_dir);
//    println!("cargo:rustc-env=PKG_CONFIG_LIBDIR={}/arm-udev/lib/pkgconfig:{}/arm-udev/usr/share/pkgconfig", manifest_dir, manifest_dir);
//    println!("cargo:rustc-env=PKG_CONFIG_SYSROOT_DIR={}/arm-udev/", manifest_dir);
//    println!("cargo:rustc-env=PKG_CONFIG_ALLOW_CROSS=1");
//
//  //  export PKG_CONFIG_PATH=${SYSROOT}/usr/lib/arm-linux-gnueabihf/pkgconfig
////export PKG_CONFIG_LIBDIR=${SYSROOT}/usr/lib/pkgconfig:${SYSROOT}/usr/share/pkgconfig
////export PKG_CONFIG_SYSROOT_DIR=${SYSROOT}
////export PKG_CONFIG_ALLOW_CROSS=1
//
//    // Config for MMAL. Allows the camera to work
//    println!("cargo:rustc-env=MMAL_INCLUDE_DIR={}/rasppi-vc/include", manifest_dir);
//    println!("cargo:rustc-env=MMAL_LIB_DIR={}/rasppi-vc/lib", manifest_dir);
//
//}

fn main() {}
