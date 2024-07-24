#!/bin/bash

. /usr/local/nvm/nvm.sh
nvm install 20
npm install -g npm@latest
 
npm install

RUST_LOG=debug npm run tauri dev
#npm run tauri dev