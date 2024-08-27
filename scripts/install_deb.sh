#!/bin/sh
set -e

# Define color variables
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to detect the NVIDIA GPU and driver version
detect_nvidia() {
    if ! command -v nvidia-smi > /dev/null 2>&1; then
        echo "${RED}nvidia-smi could not be found. Please ensure NVIDIA drivers are installed.${NC}"
        exit 1
    fi

    GPU_INFO=$(nvidia-smi --query-gpu=name,driver_version --format=csv,noheader)
    GPU_NAME=$(echo $GPU_INFO | awk -F ', ' '{print $1}')
    DRIVER_VERSION=$(echo $GPU_INFO | awk -F ', ' '{print $2}')

    echo "${GREEN}Detected GPU: $GPU_NAME${NC}"
    echo "${GREEN}Detected NVIDIA Driver Version: $DRIVER_VERSION${NC}"
}

# Function to determine the appropriate driver version
determine_driver_version() {
    case $DRIVER_VERSION in
        4[5-9][0-9].*) CUDA_VERSION="11.0" ;;
        46[0-9].*) CUDA_VERSION="11.2" ;;
        470.*) CUDA_VERSION="11.3" ;;
        510.*) CUDA_VERSION="11.6" ;;
        52[0-9].*) CUDA_VERSION="11.8" ;;
        53[0-9].*) CUDA_VERSION="11.8" ;;
        546.*) CUDA_VERSION="12.0" ;;  # Example: Mapping 546.x to CUDA 12.0
        *) 
            echo "${YELLOW}Driver version $DRIVER_VERSION is not directly supported by this script.${NC}"
            echo "Attempting to use the latest known CUDA version as a fallback."
            CUDA_VERSION="12.0"  # You can adjust this fallback version as needed.
            ;;
    esac

    echo "Determined CUDA Version: $CUDA_VERSION"
}

# Function to download and extract libcublas
download_lib() {
    LIB=$1
    VER=$2
    OS=$(uname -s)
    ARCH=$(uname -m)

    case $OS in
        Linux) OS=linux ;;
        Darwin) OS=mac ;;
        *) echo "${RED}Unsupported OS: $OS${NC}"
           exit 1 ;;
    esac

    case $ARCH in
        x86_64) ARCH=x86_64 ;;
        aarch64) ARCH=aarch64 ;;
        *) echo "${RED}Unsupported architecture: $ARCH${NC}"
           exit 1 ;;
    esac

    CUDA_URL="https://developer.download.nvidia.com/compute/cuda/redist/$LIB/$OS-$ARCH"
    CUBLAS_URL="$CUDA_URL/$LIB-$OS-$ARCH-$CUDA_$VER-archive.tar.xz"

    echo "Downloading libcublas from $CUBLAS_URL"

    mkdir -p /tmp/onceuponai
    curl -o /tmp/onceuponai/$LIB.tar.xz $CUBLAS_URL

    if [ $? -ne 0 ]; then
        echo "Failed to download $LIB."
        exit 1
    fi

    echo "Extracting $LIB..."
    xz -d /tmp/onceuponai/$LIB.tar.xz
    tar xvf /tmp/onceuponai/$LIB.tar -C /tmp/onceuponai/
    cp /tmp/onceuponai/$LIB-$OS-$ARCH-$CUDA_$VER-archive/lib/$LIB.so /usr/lib/onceuponai/

    if [ "$LIB" = "libcublas" ]; then
        cp /tmp/onceuponai/$LIB-$OS-$ARCH-$CUDA_$VER-archive/lib/"$LIB"Lt.so /usr/lib/onceuponai/
    fi

    if [ $? -ne 0 ]; then
        echo "Failed to extract $LIB."
        exit 1
    fi

    rm -r /tmp/onceuponai/"$LIB"*

    echo "${GREEN}$LIB installed successfully.${NC}"
}

# Function to display the license and prompt for acceptance
accept_license() {
    echo
    echo "${YELLOW}Please read nvidia license agreements:${NC}"
    echo "  https://developer.download.nvidia.com/compute/cuda/redist/libcurand/LICENSE.txt"
    echo "  https://developer.download.nvidia.com/compute/cuda/redist/libcublas/LICENSE.txt"
    echo
    read -p "Do you accept the NVIDIA license agreement? (yes/no): " ACCEPT

    case $ACCEPT in
        yes|y|Y|YES) echo "License accepted." ;;
        no|n|N|NO) echo "You must accept the license to proceed."
                   exit 1 ;;
        *) echo "Invalid response. Please enter yes or no."
           accept_license ;;
    esac
}

install_app() {
    echo
cat  << EOF
 ██████╗ ███╗   ██╗ ██████╗███████╗    ██╗   ██╗██████╗  ██████╗ ███╗   ██╗                  █████╗ ██╗
██╔═══██╗████╗  ██║██╔════╝██╔════╝    ██║   ██║██╔══██╗██╔═══██╗████╗  ██║                 ██╔══██╗██║
██║   ██║██╔██╗ ██║██║     █████╗      ██║   ██║██████╔╝██║   ██║██╔██╗ ██║                 ███████║██║
██║   ██║██║╚██╗██║██║     ██╔══╝      ██║   ██║██╔═══╝ ██║   ██║██║╚██╗██║                 ██╔══██║██║
╚██████╔╝██║ ╚████║╚██████╗███████╗    ╚██████╔╝██║     ╚██████╔╝██║ ╚████║    ██╗██╗██╗    ██║  ██║██║
 ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝╚══════╝     ╚═════╝ ╚═╝      ╚═════╝ ╚═╝  ╚═══╝    ╚═╝╚═╝╚═╝    ╚═╝  ╚═╝╚═╝
EOF
    echo
    echo "Installing dependencies"
    DEBIAN_FRONTEND=noninteractive
    apt update 
    apt install libwebkit2gtk-4.1-dev curl xz-utils -yq

    curl -LO https://github.com/onceuponai-dev/onceuponai/releases/download/v0.0.1-alpha.2/onceuponai_0.0.0_amd64.deb
    dpkg -i onceuponai_0.0.0_amd64.deb
}

link_libraries() {
    ln -s /usr/lib/onceuponai/libcublas.so   /usr/lib/onceuponai/libcublas.so.12
    ln -s /usr/lib/onceuponai/libcublasLt.so /usr/lib/onceuponai/libcublasLt.so.12

    ldconfig

    export LD_LIBRARY_PATH=/usr/lib/onceuponai:$LD_LIBRARY_PATH
    echo 'export LD_LIBRARY_PATH=/usr/lib/onceuponai:$LD_LIBRARY_PATH' >> ~/.bashrc
    echo "${GREEN}CUDA libraries installed successfully.${NC}"
}


# Main script execution
install_app
detect_nvidia
determine_driver_version
accept_license
download_lib libcublas 12.3.4.1
download_lib libcurand 10.3.7.37
link_libraries

#export LD_LIBRARY_PATH=/usr/lib/onceuponai:$LD_LIBRARY_PATH