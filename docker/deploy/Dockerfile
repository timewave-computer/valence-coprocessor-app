FROM rust:1.86-bookworm

RUN apt-get update && \
  apt-get install -y --no-install-recommends \
  build-essential \
  cmake \
  curl && \
  rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown && \
  rustup component add llvm-tools && \
  rustup component add rustc-dev

RUN curl -L https://sp1up.succinct.xyz | bash && \
  /root/.sp1/bin/sp1up

ADD ./dummy-circuit /tmp/dummy
RUN cd /tmp/dummy && \
  PATH="$PATH:/root/.sp1/bin" cargo prove build && \
  cd /root && \
  rm -fr /tmp/dummy

WORKDIR /usr/src/app

CMD ["/usr/src/app/docker/deploy/circuit.sh"]
