# Changelog

**dcamctl** is a command line tool to use an android device connected over USB as a webcam.

<!-- next-header -->
## [Unreleased] - TBD


## [0.2.1] - 2021-06-15

### Added

* Support for echo-cancellation on PipeWire 0.3.30+.

## [0.2.0] - 2021-05-26

### Packaging

* Removed dependency on the pulseaudio `pacmd` cli tool.

### Added

* Support for PipeWire, using its pulseaudio interface. Echo-cancellation is disabled when using PipeWire, since it doesn't support it yet.
* New `--no-audio/-n` option to disable audio setup.

### Changed

* Closing dcamctl is now done with `<Ctrl-C>` instead of `<Enter>`.

### Fixed

* Killing the process used to leave things in a inconsistent state. It now cleans things up properly when sent `SIGINT` or `SIGTERM`.

## [0.1.1] - 2021-04-23

### Added

* Support for a configuration file at `$XDG_CONFIG_HOME/dcamctl/config.yml` to set the device, port and resolution, with a `--config` option to override its location.

## [0.1.0] - 2021-04-14

### Added

* Initial support for audio and video. Only USB-connected devices are supported.
* New `--device/-d` option to change the v4l2loopback video device to use.
* New `--port/p` option to change the local port to forward between the device and the loopback interface with adb.
* New `--resolution/r` option to change the resolution.


<!-- next-url -->
[Unreleased]: https://github.com/gourlaysama/dcamctl/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/gourlaysama/dcamctl/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/gourlaysama/dcamctl/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/gourlaysama/dcamctl/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/gourlaysama/dcamctl/compare/a6e91ef...v0.1.0