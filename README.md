# Fantasy shop adventure assistant


```
az://<container>/<path>
adl://<container>/<path>
abfs://<container>/<path>
```

```
AZURE_STORAGE_ACCOUNT_NAME="devstoreaccount1" 
AZURE_STORAGE_ACCOUNT_KEY="**""
AZURE_STORAGE_USE_EMULATOR="false"
AZURE_STORAGE_USE_HTTP="true"
```

```
cp /usr/local/cuda-12/targets/x86_64-linux/lib/libcublasLt.so.12 ../../bin/
```

```
docker run -it --rm --gpus all -v $(pwd)/wheels:/wheels python:3.10.12-slim /bin/bash
docker run -it --rm --gpus all -v $(pwd)/target/wheels:/wheels nvidia/cuda:12.3.1-devel-ubuntu22.04 /bin/bash
```

```
python -m asyncio
```

```
######### CUDA ##########
sudo apt update
sudo apt upgrade

sudo apt install ubuntu-drivers-common
sudo ubuntu-drivers devices

sudo ubuntu-drivers autoinstall
# sudo apt install nvidia-driver-535

sudo restart

nvidia-smi

sudo apt install gcc

wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update
sudo apt-get -y install cuda

#optional
sudo apt --fix-broken install

sudo restart

######### Docker #########
sudo apt install -y apt-transport-https ca-certificates curl software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
sudo add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
sudo apt update
sudo apt install -y docker-ce

######## Docker Cuda ###########
sudo usermod -aG docker ${USER}

distribution=$(. /etc/os-release;echo $ID$VERSION_ID) \
  && curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add - \
  && curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | sudo tee /etc/apt/sources.list.d/nvidia-docker.list
sudo apt update
sudo apt install -y nvidia-container-toolkit
sudo systemctl restart docker
```