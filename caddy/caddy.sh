#!/bin/bash

docker run -it -p 9085:9085 --name caddy --rm --network app_default  -v $(pwd)/Caddyfile:/opt/Caddyfile caddy:2.8.4-alpine caddy run --config /opt/Caddyfile