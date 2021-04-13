# dcam

**dcam** is a command line tool to use an android device connected over USB as a webcam.

It uses adb to talk to the device, gstreamer and pulseaudio to handle the audio and video streams, and v4l2loopback to expose the video to applications as a virtual webcam.

## Usage

TODO


## Building from source

dcam is written in Rust, so you need a [Rust install] to build it. dcam compiles with
Rust 1.50 or newer.

Build from source with:

```sh
$ git clone https://github.com/gourlaysama/dcam
$ cd dcam
$ cargo build --release
$ ./target/release/dcam --version
dcam 0.1.0-dev
```

## Options

```
TODO

-h, --help           
        Prints help information.

-V, --version        
        Prints version information.
```

#### License

<sub>
dcam is licensed under either of <a href="LICENSE-APACHE">Apache License, Version 2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sub>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in dcam by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
</sub>

[Rust install]: https://www.rust-lang.org/tools/install