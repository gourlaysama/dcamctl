# Changelog

**dcamctl** is a command line tool to use an android device connected over USB as a webcam.

<!-- next-header -->
## [Unreleased] - TBD


## [0.1.0] - 2021-04-14

### Added

* Initial support for audio and video. Only USB-connected devices are supported.
* New `--device/-d` option to change the v4l2loopback video device to use.
* New `--port/p` option to change the local port to forward between the device and the loopback interface with adb.
* New `--resolution/r` option to change the resolution.


<!-- next-url -->
[Unreleased]: https://github.com/gourlaysama/dcamctl/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/gourlaysama/dcamctl/compare/a6e91ef...v0.1.0