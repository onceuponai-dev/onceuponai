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