MAKE_HELP_LEFT_COLUMN_WIDTH:=14
.PHONY: help build
help: ## This help.
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "\033[36m%-$(MAKE_HELP_LEFT_COLUMN_WIDTH)s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST) | sort

app: ## Test musl
	cd ./crates/onceuponai && \
	cargo run --release --features cuda -- serve --host 0.0.0.0 --loglevel info --workers 1 --device gpu \
		--quantized true --modelfile file:///home/jovyan/rust-src/Bielik/Bielik-7B-Instruct-v0.1.Q4_K_S.gguf \
		--tokenizerrepo speakleash/Bielik-7B-Instruct-v0.1 --e5modelrepo intfloat/multilingual-e5-small \
		--lancedburi az://recipesv2/vectors2 \
		--lancedbtable recipes_vectors \
		--prompttemplate "Skorzystaj z poniższych fragmentów kontekstu, aby odpowiedzieć na pytanie na końcu. Jeśli nie znasz odpowiedzi, po prostu powiedz, że nie wiesz, nie próbuj wymyślać odpowiedzi. \n Kontekst: \n {context}\n Pytanie:\n {question}"

node-main: ## Test musl
	cd ./crates/onceuponai && \
	RUST_LOG=debug cargo run --release -- apply -f ../../examples/main.yaml

node-e5: ## Test musl
	cd ./crates/onceuponai-actors-candle && \
	RUST_LOG=debug cargo run --release --features cuda -- spawn -f ../../examples/e5.yaml

node-bielik: ## Test musl
	cd ./crates/onceuponai-actors-candle && \
	RUST_LOG=debug cargo run --release --features cuda -- spawn -f ../../examples/bielik.yaml

node-bielik-win: ## Test musl
	cd ./crates/onceuponai-actors-candle && \
	RUST_LOG=debug cargo run --release --features cuda -- spawn -f ../../examples/bielik_win.yaml


test-bielik: ## Test musl
	cd ./crates/onceuponai-core && \
	cargo test --features cuda --release -- quantized::test_bielik --show-output

test-phi3: ## Test musl
	cd ./crates/onceuponai-core && \
	cargo test --features cuda --release -- quantized::test_phi3 --show-output

test-codegemma: ## Test musl
	cd ./crates/onceuponai-core && \
	cargo test --features cuda --release -- gemma::test_codegemma --show-output

test-lance: ## Test musl
	cd ./crates/onceuponai-core && \
	cargo test --features cuda --release -- llm::rag::test_lancedb_fruits5 --show-output

test-opendal: ## Test musl
	cd ./crates/onceuponai-core && \
	cargo test --features cuda --release -- llm::rag::test_opendal1 --show-output

build-ui: ## Test musl
	cd ./onceuponai-ui && \
	npm run build

build-linux: ## Test musl
	cd ./crates/onceuponai && \
	npm run tauri build

build-win: ## Test musl
	cd ./crates/onceuponai && \
	npm run tauri build -- --target x86_64-pc-windows-gnu

rebuild-nsis: ## Test musl
	awk '/; Copy external binaries/ {print; print "    File /a \"/oname=WebView2Loader.dll\" \"/home/jovyan/rust-src/onceuponai/target/x86_64-pc-windows-gnu/release/WebView2Loader.dll\""; next}1' ./target/x86_64-pc-windows-gnu/release/nsis/x64/installer.nsi > ./target/x86_64-pc-windows-gnu/release/nsis/x64/temp_file && \
	mv ./target/x86_64-pc-windows-gnu/release/nsis/x64/temp_file ./target/x86_64-pc-windows-gnu/release/nsis/x64/installer.nsi && \
	awk '/; Delete external binaries/ {print; print "    Delete \"$$INSTDIR\WebView2Loader.dll\""; next}1' ./target/x86_64-pc-windows-gnu/release/nsis/x64/installer.nsi > ./target/x86_64-pc-windows-gnu/release/nsis/x64/temp_file && \
	mv ./target/x86_64-pc-windows-gnu/release/nsis/x64/temp_file ./target/x86_64-pc-windows-gnu/release/nsis/x64/installer.nsi && \
	makensis ./target/x86_64-pc-windows-gnu/release/nsis/x64/installer.nsi

build-mrs: ## 
	cd ./crates/onceuponai-actors-mistralrs && \
	cargo build --release --features cuda

build-sidecar-mistralrs-cuda-linux: ## 
	cd ./crates/onceuponai-actors-mistralrs && \
	cargo build --release --features cuda && \
	cp ../../target/release/onceuponai-actors-mistralrs ../onceuponai/src-tauri/binaries/sidecar/onceuponai-actors-candle-mistralrs-x86_64-unknown-linux-gnu


build-sidecar-candle-cuda-linux: ## 
	cd ./crates/onceuponai-actors-candle && \
	cargo build --release --features cuda && \
	cp ../../target/release/onceuponai-actors-candle ../onceuponai/src-tauri/binaries/sidecar/onceuponai-actors-candle-cuda-x86_64-unknown-linux-gnu

build-sidecar-candle-cpu-linux: ## 
	cd ./crates/onceuponai-actors-candle && \
	cargo build --release && \
	cp ../../target/release/onceuponai-actors-candle ../onceuponai/src-tauri/binaries/sidecar/onceuponai-actors-candle-cpu-x86_64-unknown-linux-gnu

build-sidecar-candle-cuda-win: ## 
	cd ./crates/onceuponai-actors-candle && \
	cargo build --release --target x86_64-pc-windows-gnu  && \
	cp ../../target/x86_64-pc-windows-gnu/release/onceuponai-actors-candle.exe ../onceuponai/src-tauri/binaries/sidecar/onceuponai-actors-candle-cuda-x86_64-pc-windows-gnu.exe

build-sidecar-candle-cpu-win: ## 
	cd ./crates/onceuponai-actors-candle && \
	cargo build --release --target x86_64-pc-windows-gnu && \
	cp ../../target/x86_64-pc-windows-gnu/release/onceuponai-actors-candle.exe ../onceuponai/src-tauri/binaries/sidecar/onceuponai-actors-candle-cpu-x86_64-pc-windows-gnu.exe

build-sidecars: build-sidecar-candle-cuda-linux build-sidecar-candle-cpu-linux build-sidecar-candle-cuda-win build-sidecar-candle-cpu-win
	@echo "Sidecars build completed."

build-dev: build-sidecars build-linux
	@echo "Dev build completed."

build-full: build-ui build-sidecars build-linux build-win  
	@echo "Full build completed."

build-musl: ## Test musl
	cd ./crates/onceuponai && \
	cargo install --features cuda --target x86_64-unknown-linux-musl --path .


#build-win: ## Build windows
#	cd ./crates/onceuponai && \
#	cargo build --release --target x86_64-pc-windows-gnu


pyo3-develop: ## Test musl
	cd ./crates/onceuponai-py && \
    maturin develop --release

# https://github.com/PyO3/maturin/issues/2038 
# You can already do that by passing --compatibility linux or --skip-auditwheel

pyo3-build: ## Test musl
	cd ./crates/onceuponai-py && \
    maturin build --release --compatibility manylinux2014  --skip-auditwheel


pyo3-publish: ## Test musl
	twine upload --verbose  --repository pypi ./target/wheels/*

cuda: 
	cd ./bin && \
	./crates/onceuponai serve --host 0.0.0.0 --loglevel debug --workers 1 --device gpu --port 9090 \
		--quantized true --modelfile file:///home/jovyan/rust-src/Bielik/Bielik-7B-Instruct-v0.1.Q4_K_S.gguf \
		--tokenizerrepo speakleash/Bielik-7B-Instruct-v0.1 --e5modelrepo intfloat/multilingual-e5-small \
		--lancedburi az://recipesv2/vectors \
		--lancedbtable recipes_vectors \
		--prompttemplate "Skorzystaj z poniższych fragmentów kontekstu, aby odpowiedzieć na pytanie na końcu. Jeśli nie znasz odpowiedzi, po prostu powiedz, że nie wiesz, nie próbuj wymyślać odpowiedzi. \n Kontekst: \n {context}\n Pytanie:\n {question}"

