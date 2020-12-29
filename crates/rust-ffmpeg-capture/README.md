## Deps

### Linux

    libavcodec-dev
    libavdevice-dev
    libavfilter-dev
    libavformat-dev
    libpostproc-dev
    libswresample-dev
    libswscale-dev
    ffmpeg
    clang

If you get an operation not permitted error, try adding yourself
to the list of video users:

    usermod -aG video your_username

### Raspberry pi

You can get operation errors if the device runs out of GPU memory. see:

https://raspberrypi.stackexchange.com/questions/77070/raspberry-pi-3-and-v4l2-c-capture-code-for-jpegs/77410#77410?newreg=f991c07be9fe402fba449437abb396f0

tldr; reduce the capture resolution and try again.

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
