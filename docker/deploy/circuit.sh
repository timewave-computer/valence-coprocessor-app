#!/bin/bash

source /root/.bashrc

cd /usr/src/app/docker/build/program-circuit/program && \
  PATH="$PATH:/root/.sp1/bin" cargo prove build && \
  cd /usr/src/app/docker/build/program-circuit/script && \
  cargo run