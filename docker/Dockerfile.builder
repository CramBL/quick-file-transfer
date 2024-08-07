FROM rust:1.79-alpine

# Add the target for static linking with musl
RUN rustup target add x86_64-unknown-linux-musl
RUN apk add --no-cache --update \
    sudo \
    openssh \
    bash \
    openssh-keygen \
    musl-dev

# Setup environment variables
ENV USERNAME=qftbuilder
ENV HOME=/home/${USERNAME}

ARG USER_ID

# Create user and group
RUN addgroup ${USERNAME} \
    && adduser --shell /bin/ash --disabled-password --home $HOME --uid $USER_ID --ingroup $USERNAME $USERNAME

USER ${USERNAME}

WORKDIR /usr/src/qft

CMD ["cargo", "build", "--target", "x86_64-unknown-linux-musl"]
