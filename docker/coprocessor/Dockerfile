FROM rust:1.86-bookworm

RUN apt-get update && \
  apt-get install -y --no-install-recommends \
  build-essential \
  cmake \
  curl \ 
  git && \
  rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown && \
  rustup component add llvm-tools && \
  rustup component add rustc-dev

RUN curl -L https://sp1up.succinct.xyz | bash && \
  /root/.sp1/bin/sp1up

RUN cd /root && \
  git clone https://github.com/timewave-computer/valence-coprocessor.git && \
  cd valence-coprocessor && \
  git checkout tags/v0.1.10 && \
  cargo build -p valence-coprocessor-service --profile optimized

EXPOSE 37281
ENV RUST_LOG="info,valence_coprocessor=debug"
CMD ["cargo", "run", "--profile", "optimized", "--manifest-path", "/root/valence-coprocessor/crates/endpoint/service/Cargo.toml"]