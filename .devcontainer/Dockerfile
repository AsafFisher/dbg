# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.233.0/containers/rust/.devcontainer/base.Dockerfile

# [Choice] Debian OS version (use bullseye on local arm64/Apple Silicon): buster, bullseye
ARG VARIANT="buster"
FROM mcr.microsoft.com/vscode/devcontainers/rust:0-${VARIANT}

RUN rm -rf /usr/local/python && \
    mkdir /opt/python && \
    cd /opt/python && \
    wget -c https://www.python.org/ftp/python/3.10.0/Python-3.10.0.tar.xz &&\
    tar -Jxvf Python-3.10.0.tar.xz &&\
    cd Python-3.10.0 &&\
    ./configure --enable-shared --prefix=/usr/local &&\
    sudo make altinstall && \
    ln -s /usr/local/bin/python3.10 /usr/local/bin/python

ENV PATH="/usr/local/lib:/usr/local/bin:${PATH}"


# [Optional] Uncomment this section to install additional packages.
# RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
#     && apt-get -y install --no-install-recommends <your-package-list-here>
