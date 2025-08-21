FROM ubuntu:noble

SHELL ["bash", "-c"]

RUN apt-get update && apt-get upgrade -y && apt-get install -y build-essential curl
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && echo '. "$HOME/.cargo/env"' >> ~/.bashrc
RUN . "$HOME/.cargo/env" && rustup install 1.89 && rustup override set 1.89
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash -s -- -y
RUN . "$HOME/.nvm/nvm.sh" && nvm install 24 && nvm use 24 && nvm alias default 24 && which npm > ~/bla.txt

ENTRYPOINT ["bash"]
