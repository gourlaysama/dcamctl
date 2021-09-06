# dcamctl

**dcamctl** is a command line tool to use an android device connected over USB as a webcam.

It uses adb to talk to the device, gstreamer and pulseaudio to handle the audio and video streams, and v4l2loopback to expose the video to applications as a virtual webcam. On the device side, it is compatible with [IP Webcam].

## Installation

If you are a **Fedora** (32+) user, you can install dcamctl with:

```sh
sudo dnf copr enable gourlaysama/dcamctl
sudo dnf install dcamctl
```

Otherwise you will need to [build from source](#building-from-source).

## Usage

dcamctl exposes the audio and video from an android device with [IP Webcam] (connected over USB) as a webcam and a virtual microphone that can be used by applications like Skype, Zoom, etc. and by browser-based solutions.

### Examples

```sh
# stream 720p camera
dcamctl -r '1280x720'

# after picking a custom port in IP Webcam, set it here
dcamctl -p 8086

# v4l2loopback may have created a video device with a different name,
# for example if there already is a webcam
dcamctl -d /dev/video1
```

### Requirements

dcamctl requires to run:

- the `v4l2loopback` kernel module installed and running,
- gstreamer 1.10+,
- the Android platform tool `adb` ,
- pulseaudio and its utility tool `pactl` (PipeWire's pulseaudio compatibility layer is also supported).

On a modern Linux distributions, all the above are usually available as packages, except possibly the [`v4l2loopback` kernel module][1]. See the link for details.

dcamctl also requires [IP Webcam] on the Android device, and it being set up for debugging over USB (see online, [for example here]).

### How it works

What roughly happens:

1. the video and audio get captured and locally streamed by the IP Webcam app;
2. they get forwarded to the computer via Android USB debugging;
3. gstreamer pulls from those local streams, demuxes and decodes them in sync;
4. gstreamer converts and scales the video, then pushes it to v4l2loopback;
5. gstreamer converts the audio, then pushes it to pulseaudio;
6. v4l2loopback just reexposes what it receives as a standard v4l2 video device (a Virtual Webcam);
7. pulseaudio exposes the audio as a source (a Virtual Microphone), with echo-cancellation.

## Building from source

dcamctl is written in Rust, so you need a [Rust install] to build it. dcamctl compiles with
Rust 1.51 or newer.

Building dcam requires gstreamer 1.0 and its required gtk packages (`libgstreamer1.0-dev` on Ubuntu, `gstreamer1-devel` on Fedora).

Build from source with:

```sh
$ git clone https://github.com/gourlaysama/dcamctl -b v0.4.0
$ cd dcamctl
$ cargo build --release
$ ./target/release/dcamctl --version
dcamctl 0.4.0
```

## Options

```
--config <config>
    Use the given configuration file instead of the default.

    By default, dcamctl looks for a configuration file in
    "$XDG_CONFIG_HOME/dcamctl/config.yml" or "$HOME/.config/dcamctl/config.yml".

-d, --device <device>
    v4l2loopback video device to use.

    This device must be one expose by the v4l2loopback kernel module. Check the devices
    under /dev/video* with `v4l2-ctl -d /dev/videoX -D` for the correct one.
    [default: /dev/video0]

-f, --flip <flip>
    Flip method used to mirror the video.

    Defaults to none. [possible values: horizontal, vertical, none]

-p, --port <port>
    Port to forward between the device and localhost.

    The port on on the device with this value will be forwarded to the same port on
    localhost. [default: 8080]

-r, --resolution <resolution>
    Output resolution to use.

    The video feed will be resized to this value if needed. [default: auto]

-n, --no-audio
    Disable audio support.

    Do not setup audio forwarding or interact at all with the audio system.

-h, --help
    Prints help information

-q, --quiet
    Pass for less log output

-V, --version
    Prints version information

-v, --verbose
    Pass for more log output
```

## Configuration

dcamctl doesn't create a configuration file for you, but looks for it in in `$XDG_CONFIG_HOME/dcamctl/config.yml` or `$HOME/.config/dcamctl/config.yml`. See the default configuration file at [`config.yml`][2] for an example.

### Configuration keys

- `port` (number): the port to forward between the device and localhost (can be overriden on the command-line with `-p/--port`).
- `device` (string): the v4l2loopback video device to use (can be overriden on the command-line with `-d/--device`).
- `resolution` (string): the output resolution to use (can be overriden on the command-line with `-r/--resolution`). Can be set to a pair like `640x480`, or make dcamctl autodetect the resolution with `auto`.
- `no_audio` (boolean): if true, disable audio support (can be overriden on the command-line with `-n/--no-audio`).
- `flip` (string): the method used to mirror the video, from `horizontal`, `vertical` or `none` (can be overriden on the command-line with `-f/--flip`).

---

#### License

<sub>
dcamctl is licensed under the <a href="LICENSE-APACHE">Apache License, Version 2.0</a>.
</sub>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in dcamctl by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.
</sub>

[rust install]: https://www.rust-lang.org/tools/install
[ip webcam]: https://play.google.com/store/apps/details?id=com.pas.webcam
[for example here]: https://joyofandroid.com/how-to-enable-usb-debugging-on-android/
[1]: https://github.com/umlaeute/v4l2loopback
[2]: https://github.com/gourlaysama/dcamctl/blob/v0.4.0/config.yml
