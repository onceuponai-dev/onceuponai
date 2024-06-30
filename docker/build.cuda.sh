#!/bin/bash

mkdir onceuponai-ui
cp -r ../onceuponai-ui/src onceuponai-ui/
cp -r ../onceuponai-ui/public onceuponai-ui/
cp -r ../onceuponai-ui/*.json onceuponai-ui/
cp -r ../onceuponai-ui/*.ts onceuponai-ui/
cp -r ../onceuponai-ui/index.html onceuponai-ui/

mkdir onceuponai-backend
mkdir onceuponai-backend/onceuponai

cp -r ../onceuponai/src onceuponai-backend/onceuponai/
cp -r ../onceuponai/ui onceuponai-backend/onceuponai/
cp -r ../onceuponai/Cargo.toml onceuponai-backend/onceuponai/


mkdir onceuponai-backend/onceuponai-core
cp -r ../onceuponai-core/src onceuponai-backend/onceuponai-core/
cp -r ../onceuponai-core/Cargo.toml onceuponai-backend/onceuponai-core/

mkdir onceuponai-backend/onceuponai-operator
cp -r ../onceuponai-operator/src onceuponai-backend/onceuponai-operator/
cp -r ../onceuponai-operator/Cargo.toml onceuponai-backend/onceuponai-operator/

mkdir onceuponai-backend/onceuponai-py
cp -r ../onceuponai-py/src onceuponai-backend/onceuponai-py/
cp -r ../onceuponai-py/Cargo.toml onceuponai-backend/onceuponai-py/



cp -r ../Cargo.toml onceuponai-backend/
cp -r ../Cargo.lock onceuponai-backend/



docker buildx build --platform linux/amd64 -t qooba/onceuponai:cuda -f Dockerfile.cuda .

rm -r onceuponai-ui
rm -r onceuponai-backend