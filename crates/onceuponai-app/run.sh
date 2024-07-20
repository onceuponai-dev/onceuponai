#!/bin/bash

. /usr/local/nvm/nvm.sh
nvm install 20
npm install -g npm@latest
 
npm install
npm run tauri dev