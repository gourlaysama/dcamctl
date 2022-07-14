% DCAMCTL(1) Version 0.4.3 | Dcamctl Usage Documentation

NAME
====

**dcamctl** — use an android device as a webcam with v4l2loopback

SYNOPSIS
========

| **dcamctl** \[_OPTIONS_]...
| **dcamctl** \[**-h**|**\--help**|**-V**|**\--version**]

DESCRIPTION
===========

Use an android device as a webcam with v4l2loopback.

OPTIONS
=======

Query options
-------------

\--config _FILE_

:   Use the given configuration file instead of the default.

    By default, dcamctl looks for a configuration file in
    _`$XDG_CONFIG_HOME/dcamctl/config.yml`_ or _`$HOME/.config/dcamctl/config.yml`_.

-d, \--device _DEVICE_

:   v4l2loopback video device to use.

    This device must be one expose by the v4l2loopback kernel module. Check the devices
    under _`/dev/video*`_ with _`v4l2-ctl -d /dev/videoX -D`_ for the correct one.
    The default is _`/dev/video0`_

    This option overrides the corresponding value from the config.

-f, \--flip _FLIP_METHOD_

:   Flip method used to mirror the video.

    Possible values are _`horizontal`_, _`vertical`_ and _`none`_. The default is _`none`_.

    This option overrides the corresponding value from the config.

 -p, \--port _PORT_

:   Port to forward between the device and localhost.

    The port on on the device with this value will be forwarded to the same port on
    localhost. The default is _`8080`_.

    This option overrides the corresponding value from the config.

-r, \--resolution _RESOLUTION_

:   Output resolution to use.

    The video feed will be resized to this value if needed.

    Possible values are _`auto`_ or a display resolution in _`width x height`_ format like _`1024x768`_.
    The default is _`auto`_.

    This option overrides the corresponding value from the config.

-s, \--serial  _ANDROID_SERIAL_

:   Connect to android device with the given serial.

    This option overrides the corresponding value from the config.

Flags
-----

-n, \--no-audio

:   Disable audio support.

    Do not setup audio forwarding or interact at all with the audio system.

-C, \--no-echo-cancel

:   Disable echo-canceling.

-q, \--quiet

:   Pass for less log output

-v, \--verbose

:   Pass for more log output

Info
----

-h, \--help

:   Print help information

-V, \--version

:   Print version information

FILES
=====

_\$XDG_CONFIG_HOME/dcamctl/config.yml_ or _\$HOME/.config/dcamctl/config.yml_

:   Default configuration file.

BUGS
====

See GitHub Issues: <https://github.com/gourlaysama/dcamctl/issues>

AUTHOR
======

Antoine Gourlay <antoine@gourlay.fr>
