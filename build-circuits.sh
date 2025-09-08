#!/usr/bin/env bash
set -e

if command -v nix >/dev/null; then
  nix --extra-experimental-features "nix-command flakes" run -- "$@"
  exit 0
elif docker info >/dev/null 2>&1; then
  ENGINE=docker
  echo "Setting up nix within docker to build circuits"
elif command -v podman >/dev/null 2>&1; then
  ENGINE=podman
  echo "Setting up nix within podman to build circuits"

  podman machine init 2>/dev/null || true
  podman machine start 2>/dev/null || true
else
  echo "Error: unable to access nix, docker or podman"
  echo "Building circuits reproducibly requires access to nix natively or access to docker or podman to setup nix within a container"
  exit 1
fi 

if $ENGINE image inspect nix-circuit-builder >/dev/null 2>&1; then
  echo Loading existing builder image: nix-circuit-builder
  $ENGINE create --name nix-circuit-builder --platform linux/amd64 -v "$(pwd)":/code -w /code -ti nix-circuit-builder bash
else 
  $ENGINE create --name nix-circuit-builder \
    --platform linux/amd64 -v "$(pwd)":/code -w /code -ti nixpkgs/nix-flakes sh -c \
    "echo filter-syscalls = false >> /etc/nix/nix.conf && git config --global --add safe.directory '*' && exec bash"
fi
function cleanup {
  echo "Saving build state to image: nix-circuit-builder"
  $ENGINE commit nix-circuit-builder nix-circuit-builder
  $ENGINE stop nix-circuit-builder
  $ENGINE rm nix-circuit-builder
  if [[ "$ENGINE" == "podman" ]]; then
    $ENGINE machine stop 2>/dev/null || true
  fi
}
trap cleanup EXIT
$ENGINE start nix-circuit-builder
$ENGINE exec -t -e NIX_ARGS="$NIX_ARGS" nix-circuit-builder nix run
