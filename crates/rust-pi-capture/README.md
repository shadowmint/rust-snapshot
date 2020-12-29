## Check

    MMAL_LIB_DIR=`pwd`/ref/mmal cargo check --target=armv7-unknown-linux-gnueabihf

## Not working?

It doesn't work if v4l2 is enabled. Use:

    sudo rmmod bcm2835-v4l2