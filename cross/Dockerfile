FROM rustembedded/cross:armv7-unknown-linux-gnueabihf

RUN dpkg --add-architecture armhf && \
    apt-get update && \
    apt-get install --assume-yes libssl-dev:armhf

ENV OPENSSL_DIR=/usr/include/arm-linux-gnueabihf/openssl \
    OPENSSL_INCLUDE_DIR=/usr/include/arm-linux-gnueabihf \
    OPENSSL_LIB_DIR=/usr/lib/arm-linux-gnueabihf/
