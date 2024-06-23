#!/bin/bash
. ~/.nvm/nvm.sh
nvm install 20
npm install -g npm@latest

npm run dev -- --host 0.0.0.0 --port 8082
