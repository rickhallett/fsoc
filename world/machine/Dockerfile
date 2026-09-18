# A generic "machine" in the fsoc world: real Debian, sshd, the fsoc toolkit,
# a themed operator shell, and a shared ops key so you can pivot host->host
# passwordlessly (like a real jump setup). Each host in the compose network is
# this image with its own hostname and hardware profile.
FROM debian:bookworm-slim

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        openssh-server openssh-client \
        coreutils findutils grep sed gawk less vim procps \
        iproute2 iputils-ping ca-certificates curl unzip file git \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /run/sshd \
    && sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config \
    && sed -i 's/^#\?PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config

# fsoc toolkit (off-PATH; players opt in with `fsoc install`)
COPY fsoc/ /opt/fsoc/
RUN chmod +x /opt/fsoc/fetch-tools.sh /opt/fsoc/fsoc \
    && /opt/fsoc/fetch-tools.sh \
    && ln -sf /opt/fsoc/fsoc /usr/local/bin/fsoc

COPY machine/setup.sh /opt/machine/setup.sh
COPY machine/entrypoint.sh /opt/machine/entrypoint.sh
COPY setup/theme.bashrc /opt/machine/theme.bashrc
RUN chmod +x /opt/machine/setup.sh /opt/machine/entrypoint.sh && /opt/machine/setup.sh

EXPOSE 22
CMD ["/opt/machine/entrypoint.sh"]
