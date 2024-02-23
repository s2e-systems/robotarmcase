The `dobot` crate is 2 years old and uses a deprecated feature which prevents building with the current version of Rust. I made a fork which removes this feature at https://bitbucket.org/s2e-systems/dobot/src/remove-fixed-size-array/.

In order to cross-compile for Raspberry, some extra steps are required:

1. Have the armhf version of `libudev` installed. For Ubuntu, see: https://github.com/dcuddeback/libudev-sys/issues/7#issuecomment-786952047
2. Set the following environment variables:
```sh
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig/
```

In order to be able to run the program, the serial port must be activated (see https://raspberrypi.stackexchange.com/a/133037).