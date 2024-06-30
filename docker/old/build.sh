#!/bin/bash

cp -r ../target/wheels ./wheels
docker build -t qooba/onceuponai:recipes .

rm -r wheels