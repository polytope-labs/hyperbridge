#!/bin/bash

if [ ! -d "$HOME/.local/bin" ] ; then
  mkdir -p "$HOME/.local/bin"
  printf '\nexport "PATH=$PATH:'"$HOME"/.local/bin'"' >> "$HOME/.bashrc"
  # Change the PATH right now:
  PATH="$PATH:$HOME/.local/bin"
fi

wget -O $HOME/.local/bin/zombienet -q --show-progress https://github.com/paritytech/zombienet/releases/download/v1.3.89/zombienet-linux-x64
