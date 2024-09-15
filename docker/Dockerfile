ARG CUBLAS_VERSION="linux-x86_64-12.3.4.1-archive"
ARG CURAND_VERSION="linux-x86_64-10.3.7.37-archive"
ARG ONCEUPONAI_VERSION="v0.0.1-alpha.2"
ARG ONCEUPONAI_DEB="onceuponai_0.0.0_amd64.deb"

FROM ubuntu:24.04 as downloader

ARG CUBLAS_VERSION
ARG CURAND_VERSION
ARG ONCEUPONAI_VERSION
ARG ONCEUPONAI_DEB

RUN apt-get update && apt-get install -y wget xz-utils
WORKDIR /opt/onceuponai
RUN wget https://github.com/onceuponai-dev/onceuponai/releases/download/${ONCEUPONAI_VERSION}/${ONCEUPONAI_DEB}
WORKDIR /opt/cuda
RUN wget https://developer.download.nvidia.com/compute/cuda/redist/libcublas/linux-x86_64/libcublas-${CUBLAS_VERSION}.tar.xz
RUN wget https://developer.download.nvidia.com/compute/cuda/redist/libcurand/linux-x86_64/libcurand-${CURAND_VERSION}.tar.xz

RUN xz -d libcublas-${CUBLAS_VERSION}.tar.xz && \
    tar xvf libcublas-${CUBLAS_VERSION}.tar && \
    rm libcublas-${CUBLAS_VERSION}.tar && \
    xz -d libcurand-${CURAND_VERSION}.xz && \
    tar xvf libcurand-${CURAND_VERSION}.tar && \
    rm libcurand-${CURAND_VERSION}.tar

FROM ubuntu:24.04

ARG CUBLAS_VERSION
ARG CURAND_VERSION
ARG ONCEUPONAI_VERSION
ARG ONCEUPONAI_DEB

COPY --from=downloader /opt/onceuponai /opt/onceuponai
RUN apt update && apt install libwebkit2gtk-4.1-0 libgtk-3-0 -y
RUN dpkg -i /opt/onceuponai/${ONCEUPONAI_DEB}
RUN rm /opt/onceuponai
RUN rm -rf /var/lib/apt/list/*

COPY --from=downloader /opt/cuda/libcublas-${CUBLAS_VERSION}/lib/libcublas.so   /usr/lib/onceuponai/
COPY --from=downloader /opt/cuda/libcublas-${CUBLAS_VERSION}/lib/libcublasLt.so /usr/lib/onceuponai/
COPY --from=downloader /opt/cuda/libcurand-${CURAND_VERSION}/lib/libcurand.so  /usr/lib/onceuponai/

RUN ln -s /usr/lib/onceuponai/libcublas.so   /usr/lib/onceuponai/libcublas.so.12 && \
    ln -s /usr/lib/onceuponai/libcublasLt.so /usr/lib/onceuponai/libcublasLt.so.12

ENV LD_LIBRARY_PATH=/usr/lib/onceuponai:$LD_LIBRARY_PATH
EXPOSE 8080
ENTRYPOINT ["onceuponai"]
