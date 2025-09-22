FROM ubuntu:noble AS builder
SHELL ["bash", "-c"]
ENV BASH_ENV=/etc/bashenv
RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y build-essential curl
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN echo '. "$HOME/.cargo/env"' >> /etc/bashenv
RUN rustup install 1.90
RUN rustup override set 1.90
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash -s -- -y
RUN echo 'export NVM_DIR="$HOME/.nvm"' >> /etc/bashenv
RUN echo '. "$NVM_DIR/nvm.sh"' >> /etc/bashenv
RUN nvm install 24
RUN nvm use 24
RUN nvm alias default 24

FROM builder AS build
SHELL ["/bin/bash", "-c"]
WORKDIR /src
COPY . /src
RUN cd /src/frontend && npm install
RUN cd /src && cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=build /src/target/release/image-watch /
CMD ["/image-watch"]
