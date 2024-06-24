FROM alpine:latest

# Setup environment variables
ENV USERNAME=userfoo
ENV PASSWORD="password"
ENV HOME=/home/$USERNAME

RUN apk add --no-cache --update \
    sudo \
    openssh \
    bash \
    openssh-keygen \
    curl

# Set timezone to Europe/London
RUN echo 'Europe/London' > /etc/timezone

# Create user and group
RUN addgroup $USERNAME \
    && adduser --shell /bin/ash --disabled-password --home $HOME --ingroup $USERNAME $USERNAME \
    && echo "${USERNAME}:${PASSWORD}" | chpasswd

# Generate SSH keys and configure SSH
RUN ssh-keygen -A \
    && ssh-keygen -t dsa -b 1024 -N '' -f /etc/ssh/ssh_host_dsa_key \
    && echo "HostKey /etc/ssh/ssh_host_rsa_key" >> /etc/ssh/sshd_config \
    && echo "HostKey /etc/ssh/ssh_host_ed25519_key" >> /etc/ssh/sshd_config \
    && echo "HostKey /etc/ssh/ssh_host_ecdsa_key" >> /etc/ssh/sshd_config \
    && sed -i -E "s|(AuthorizedKeysFile).*|\1 %h/.ssh/authorized_keys|g" /etc/ssh/sshd_config \
    && sed -i -E "s/#?(ChallengeResponseAuthentication|PasswordAuthentication).*/\1 yes/g" /etc/ssh/sshd_config

# Create .ssh directory and authorize keys
RUN mkdir -p $HOME/.ssh \
    && umask 066 \
    && touch $HOME/.ssh/authorized_keys \
    && ssh-keygen -t rsa -b 4096 -m pem -N '' -f /root/rsa && cat /root/rsa.pub >> $HOME/.ssh/authorized_keys \
    && ssh-keygen -t ed25519 -N '' -f /root/ed25519 && cat /root/ed25519.pub >> $HOME/.ssh/authorized_keys \
    && chown -R $USERNAME $HOME/.ssh

COPY target/x86_64-unknown-linux-musl/debug/qft /usr/bin/qft

# Run the docker daemon but don't detach (-D) and listen on port 54320
CMD ["/usr/sbin/sshd", "-D", "-p", "54320"]
