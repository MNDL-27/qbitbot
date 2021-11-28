#!/bin/bash

mkdir -p config
mkdir -p downloads
docker-compose up -d
cnt=0
while [[ "$cnt" -lt 5 ]]; do
  sleep 1
  curl http://localhost:8080 &> /dev/null && exit 0
  echo "waiting for Qbittorent to start"
  cnt=$((cnt+1))
done
exit 1