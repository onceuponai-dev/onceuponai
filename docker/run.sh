#!/bin/bash

docker run -it --rm \
 -v $(pwd)/../../../../../huggingface:/home/jovyan/.cache/huggingface \
 --gpus all onceuponai/onceuponai:recipes /bin/bash