#!/bin/sh
set -e

docker build -t tarinu/taribot-rs:arm32v7 --build-arg ARCH=arm32v7 .
docker build -t tarinu/taribot-rs:amd64 --build-arg ARCH=amd64 .

docker push tarinu/taribot-rs:arm32v7
docker push tarinu/taribot-rs:amd64

docker manifest create tarinu/taribot-rs:latest tarinu/taribot-rs:amd64 tarinu/taribot-rs:arm32v7
docker manifest push --purge tarinu/taribot-rs:latest
