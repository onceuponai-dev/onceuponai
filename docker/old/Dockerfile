# based on https://dev.to/kummerer94/multi-stage-docker-builds-for-pyton-projects-using-uv-223g
FROM python:3.10.12 as build

RUN apt-get update && apt-get install -y build-essential curl
ENV VIRTUAL_ENV=/opt/venv \
    PATH="/opt/venv/bin:$PATH"

ADD https://astral.sh/uv/install.sh /install.sh
RUN chmod -R 655 /install.sh && /install.sh && rm /install.sh
COPY ./requirements.txt .
COPY ./wheels /wheels
RUN /root/.cargo/bin/uv venv /opt/venv
RUN /root/.cargo/bin/uv pip install --no-cache -r requirements.txt
RUN /root/.cargo/bin/uv pip install --no-cache /wheels/onceuponai-0.0.2-cp310-cp310-manylinux_2_34_x86_64.whl

FROM python:3.10.12-slim-bookworm
COPY --from=build /opt/venv /opt/venv

# CREATE USER
RUN useradd -ms /bin/bash jovyan
ARG NB_USER=jovyan
ARG NB_UID=1000
ARG NB_GID=100

USER 1000
WORKDIR /home/jovyan

ENV PATH="/opt/venv/bin:$PATH"
COPY ./serve_recipes.py .
ENTRYPOINT ["serve", "run", "serve_recipes:recipes_app"]