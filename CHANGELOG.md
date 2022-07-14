# Changelog

**dcamctl** is a command line tool to use an android device connected over USB as a webcam.

<!-- next-header -->
## [Unreleased] - TBD

### Features

* New `--serial/-s` option and configuration value to give a custom android serial to use with adb. If unset, adb will be called without serial, which will throw an error if there is more than one device connected.

### Packaging

* The Minimum Supported Rust Version is now 1.57.
* Shell completions are now generated (for bash, zsh and fish) and provided on the release page and in the COPR package.
* There is now a man page for girouette available on the release page (generated with pandoc, from `doc/dcamctl.1.md`).

## [0.4.3] - 2022-06-15

### Security

* Update dependencies to fix [CVE-2021-45710], [CVE-2022-24713].

## [0.4.2] - 2021-10-27

### Changes

* `--version` output now shows more build information and the default location of the configuration file.

## [0.4.1] - 2021-09-08

### Packaging

* fix documentation: dcamctl is licensed under the Apache License v2.0 only.

### Features

* New `--no-echo-cancel/-C` option to force disable echo-canceling.

## [0.4.0] - 2021-09-06

### Packaging

* The Minimum Supported Gstreamer Version is now 1.10.
* Building dcamctl now requires the development headers for gstreamer-video (`libgstreamer-plugins-base1.0-dev` on Ubuntu, `gstreamer1-plugins-base-devel` on Fedora).

### Features

* New `--flip/-f <horizontal|vertical|none>` option to horizontally/vertically mirror the video. This can also be controlled from the terminal with the `f` key.

## [0.3.1] - 2021-08-09

### Security

* update dependencies to fix [RUSTSEC-2021-0078], [RUSTSEC-2021-0079], and [RUSTSEC-2021-0072].

## [0.3.0] - 2021-07-02

### Packaging

* The Minimum Supported Rust Version is now 1.51.

### Added

* The IP Webcam Android application can now be controlled from the terminal if dcamctl detects support. Available controls are:
  zoom in/out with `z/Z`, quality up/down with `t/T` and panning with direction keys.
* The `resolution` option supports the new value `auto`, in which the video resolution is automatically detected by querying the IP Webcam application, with a fallback to `640x480`.

### Changed

* The default value for `resolution` is now `auto`.

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
[Unreleased]: https://github.com/gourlaysama/dcamctl/compare/v0.4.3...HEAD
[0.4.3]: https://github.com/gourlaysama/dcamctl/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/gourlaysama/dcamctl/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/gourlaysama/dcamctl/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/gourlaysama/dcamctl/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/gourlaysama/dcamctl/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/gourlaysama/dcamctl/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/gourlaysama/dcamctl/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/gourlaysama/dcamctl/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/gourlaysama/dcamctl/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/gourlaysama/dcamctl/compare/a6e91ef...v0.1.0
[RUSTSEC-2021-0078]: https://rustsec.org/advisories/RUSTSEC-2021-0078
[RUSTSEC-2021-0079]: https://rustsec.org/advisories/RUSTSEC-2021-0079
[RUSTSEC-2021-0072]: https://rustsec.org/advisories/RUSTSEC-2021-0072
[CVE-2021-45710]: https://github.com/advisories/GHSA-fg7r-2g4j-5cgr
[CVE-2022-24713]: https://github.com/advisories/GHSA-m5pq-gvj9-9vr8
