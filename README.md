# Snapshot tools

The manfiest format is defined in `src/app/config`, but broadly should look
like this:

    [config]
    output_folder = "test/output"
    log_folder = "test/logs"
    lock_file = "test/lock"
    sample_interval = 5000
    sample_idle = 100
    
    [settings]
    backend = "avfoundation"
    resolution = "1280x720"
    framerate = "24"
    device = "0:0"
    pixel_format = "0rgb"

See `settings.test.toml` for an example using the mock camera.

Otherwise the device is created using libav and the settings provided.

You should use the ffmpeg cli to determine what the appropriate settings
for your device are.

## Dependencies

See `crates/rust-ffmpeg-capture`, but broadly speaking to use libav on
mac use `brew install ffmpeg`, on linux, you need to install:

    libavcodec-dev
    libavdevice-dev
    libavfilter-dev
    libavformat-dev
    libpostproc-dev
    libswresample-dev
    libswscale-dev
    ffmpeg
    clang

On windows install the ffmpeg libraries manually or use choco.

## Helpful ffmpeg commands

    ffmpeg -devices

Find sources:

    ffmpeg -sources video4linux2
    ffmpeg -sources avfoundation

Find devices:

    ffmpeg -sources avfoundation -devices

Take screenshot:

    ffmpeg -f video4linux2 -r 1 -s 640x480 -i "/dev/video0" -frames 1 -y test.png

or:

    ffmpeg -f avfoundation -s 1280x720 -r 24 -i "1" -frames 1 -y test.png

For linux, try:

    v4l2-ctl -d 0 --list-formats-ext

## Snapshot

    cargo run --release --bin snapshot -- settings.mac.toml

While the capture is running a 'lock' file is created; to halt the
capture process, remove the lock file.

## Assemble

    cargo run --release --bin assemble -- settings.mac.toml

This is just a wrapper around the ffmpeg cli. It will only work if 
the ffmpeg cli tools are installed.

## License

This software is MIT license, however, note that it uses libav, which is
LGPL, and portions of it are GPL.

Please read https://libav.org/legal/ for more details; it is your responsibility
to ensure that any usage complies with the relevant licenses.