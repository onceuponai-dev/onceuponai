<img src=".github/splash.png" alt="Once Upon ... AI" />

# Once Upon ... AI: Start Your AI Adventures with Ease

## Introduction

Once Upon ... AI is a rust-based desktop/server application designed to simplify the setup and usage of machine learning models (LLMs, Embedding models, Image generative models, Vision models and other). 
Whether you want to quickly test different models like LLM-s with different parameters or build scalable solution this app streamlines the entire process.
With support for both CPU and CUDA-accelerated GPU environments, you can deploy models locally or scale them across multiple nodes for production environments. 

<p align="center">
    <img src=".github/intro.gif" alt="intro" />
</p>

## Supported Models


<img src=".github/spawn_model.png" alt="spawn model" />

### Embeddings: 
* [intfloat/e5-small-v2](https://huggingface.co/intfloat/e5-small-v2)
* [intfloat/e5-large-v2](https://huggingface.co/intfloat/e5-large-v2)
* [intfloat/multilingual-e5-small](https://huggingface.co/intfloat/multilingual-e5-small)
* [intfloat/multilingual-e5-large](https://huggingface.co/intfloat/multilingual-e5-large)

### LLMs:
* [google/gemma-2b-it](https://huggingface.co/google/gemma-2b-it)
* [google/gemma-7b-it](https://huggingface.co/google/gemma-7b-it)

### Quantized LLMs:
* [speakleash/Bielik-7B-Instruct-v0.1-GGUF](https://huggingface.co/speakleash/Bielik-7B-Instruct-v0.1-GGUF)
* [TheBloke/Llama-2-7B-GGML](https://huggingface.co/TheBloke/Llama-2-7B-GGML)
* [TheBloke/Llama-2-13B-GGML](https://huggingface.co/TheBloke/Llama-2-13B-GGML)
* [TheBloke/Llama-2-7B-Chat-GGML](https://huggingface.co/TheBloke/Llama-2-7B-Chat-GGML)
* [TheBloke/Llama-2-13B-Chat-GGML](https://huggingface.co/TheBloke/Llama-2-13B-Chat-GGML)
* [TheBloke/CodeLlama-2-7B-GGML](https://huggingface.co/TheBloke/CodeLlama-2-7B-GGML)
* [TheBloke/CodeLlama-2-13B-GGML](https://huggingface.co/TheBloke/CodeLlama-2-13B-GGML)
* [TheBloke/leo-hessianai-7B-GGUF](https://huggingface.co/TheBloke/leo-hessianai-7B-GGUF)
* [TheBloke/leo-hessianai-13B-GGUF](https://huggingface.co/TheBloke/leo-hessianai-13B-GGUF)
* [TheBloke/Mixtral-8x7B-v0.1-GGUF](https://huggingface.co/TheBloke/Mixtral-8x7B-v0.1-GGUF)
* [TheBloke/Mixtral-8x7B-Instruct-v0.1-GGUF](https://huggingface.co/TheBloke/Mixtral-8x7B-Instruct-v0.1-GGUF)
* [TheBloke/Mistral-7B-v0.1-GGUF](https://huggingface.co/TheBloke/Mistral-7B-v0.1-GGUF)
* [TheBloke/Mistral-7B-Instruct-v0.1-GGUF](https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.1-GGUF)
* [TheBloke/Mistral-7B-Instruct-v0.2-GGUF](https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.2-GGUF)
* [TheBloke/zephyr-7B-alpha-GGUF](https://huggingface.co/TheBloke/zephyr-7B-alpha-GGUF)
* [TheBloke/zephyr-7B-beta-GGUF](https://huggingface.co/TheBloke/zephyr-7B-beta-GGUF)
* [TheBloke/openchat_3.5-GGUF](https://huggingface.co/TheBloke/openchat_3.5-GGUF)
* [TheBloke/Starling-LM-7B-alpha-GGUF](https://huggingface.co/TheBloke/Starling-LM-7B-alpha-GGUF)
* [QuantFactory/Meta-Llama-3-8B-GGUF](https://huggingface.co/QuantFactory/Meta-Llama-3-8B-GGUF)
* [microsoft/Phi-3-mini-4k-instruct-gguf](https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf)

## Installation

### Linux

```bash
sudo apt update
sudo apt install wget xz-utils -y

wget https://raw.githubusercontent.com/onceuponai-dev/onceuponai/tauri/scripts/install.sh
sudo bash ./install.sh
```

### Windows

For windows you can use WSL2 and run `install.sh` script as for `Linux`. 
If you have GPU card use [https://docs.nvidia.com/cuda/wsl-user-guide/index.html](https://docs.nvidia.com/cuda/wsl-user-guide/index.html)
to have CUDA support.
To see GUI will need to run X-server eg. Xming, VcXsrv, Cygwin/X, MobaXterm, WezTerm

### Mac 

TODO

### Docker

To run `Once Upon ... AI` with docker run:
```
docker run -it --name onceuponai --rm -p 8080:8080 \
        -e DISPLAY=host.docker.internal:0.0 \
        --gpus all \
        -v $(pwd)/huggingface:/home/ubuntu/.cache/huggingface \
        onceuponai/onceuponai:v0.0.1-alpha.2
```

## Usage

After spawning a model you can call it using REST API which is compatible with OpenAI API.

```python
import os
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key=os.environ["PERSONAL_TOKEN"],
)

completion = client.chat.completions.create(
  model="quantized/mistral7b-instruct-v02",
  messages=[
    {"role": "user", "content": "At what temperature does water boil ?"}
  ]
)

print(completion.choices[0].message)
```

## Architecture


<img src=".github/architecture_multinode.jpeg" alt="spawn model" />

Once Upon ... AI is built using an actors architecture, leveraging Rust's high-performance capabilities. 
The application uses the [Actix Telepathy](https://github.com/wenig/actix-telepathy) framework to implement a distributed system of actors, 
allowing each model to run as an isolated actor (process). 

This design offers several advantages:
* Scalability: Deploy models across multiple nodes or machines, ensuring high availability and performance in production environments.
* Modularity: Easily spawn or terminate model deployments via the desktop app, giving you full control over your resources.
* Gateway/Seed Node: The central gateway node exposes a REST API, allowing for seamless integration with external services and client applications.

## Authentication

REST API is secured using Personal User Tokens. 
Additionally, the web server supports integration with OIDC providers for streamlined authentication in enterprise environments.

Personal User Tokens: Each user must generate a unique token to access the API. This token ensures that only authorized users can interact with the deployed models.

<img src=".github/personal_token.png" alt="spawn model" />

OIDC Integration: For environments requiring enterprise-level security, the Actix server UI can integrate with popular OpenID Connect providers, offering a seamless and secure authentication experience.

```bash
Once Upon ... AI -

Usage: onceuponai [OPTIONS]

Options:
      --actor-host <ACTOR_HOST>                                      [default: 127.0.0.1:1992]
      --host <HOST>                                                  [default: 0.0.0.0]
      --port <PORT>                                                  [default: 8080]
      --log-level <LOG_LEVEL>                                        [default: info]
      --workers <WORKERS>                                            [default: 0]
      --invoke-timeout <INVOKE_TIMEOUT>                              [default: 60]
      --session-key <SESSION_KEY>
      --personal-access-token-secret <PERSONAL_ACCESS_TOKEN_SECRET>
      --headless
      --oidc
      --oidc-issuer-url <OIDC_ISSUER_URL>
      --oidc-client-id <OIDC_CLIENT_ID>
      --oidc-client-secret <OIDC_CLIENT_SECRET>
      --oidc-redirect-url <OIDC_REDIRECT_URL>
  -h, --help                                                         Print help
  -V, --version                                                      Print version
```



<!--
API Usage Example:

python
Copy code
import requests

# Example API call to interact with a deployed LLM model
response = requests.post(
    "http://localhost:8000/api/v1/generate",
    json={"prompt": "Once upon a time...", "model": "gemma"}
)
print(response.json())
Or use curl:

bash
Copy code
curl -X POST http://localhost:8000/api/v1/generate -H "Authorization: Bearer <TOKEN>" -d '{"prompt": "Once upon a time...", "model": "gemma"}'
Supported Models
"Once Upon ... AI" currently supports the following models:

Embedding Models:
E5
Large Language Models:
Gemma
Llama
Phi
Mistral
Vision Models:
(Add the list of supported Vision models here)
Each model comes pre-configured with appropriate parameters, making it easy to get started without diving into the complexities of setup.

Architecture

Conclusion
Once Upon ... AI is a cutting-edge tool designed to make machine learning more accessible and scalable. Whether you're a developer experimenting on your local machine or deploying complex models in a production environment, this app provides the performance, security, and flexibility you need. Dive into your AI adventures today, and let "Once Upon ... AI" simplify the story of your next project.

-->