#!/bin/bash

HOST=$1
PORT=$2

echo "Trying to connect to ${HOST}:${PORT}..."

while ! nc -z $HOST $PORT; do
  echo "Waiting for $HOST:$PORT to become available"
  sleep 0.5
done

echo "${HOST}:${PORT} is ready for requests."