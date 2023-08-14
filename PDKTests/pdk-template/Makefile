TARGET		:= wasm32-wasi
TARGET_DIR	:= target/$(TARGET)/release
NAME		:= {{crate_name}}

.phony: build
build: manifest.yaml
	@cargo build --target $(TARGET) --release
	@cat $(TARGET_DIR)/$(NAME).wasm | base64 | tr -d '\n\r' > $(TARGET_DIR)/$(NAME).b64
	@awk 'BEGIN{getline l < "$(TARGET_DIR)/$(NAME).b64"}/<ENCODED>/{gsub("<ENCODED>",l)}1' manifest.yaml > $(TARGET_DIR)/$(NAME).yaml

.phony: deploy
deploy: build
	cp $(TARGET_DIR)/$(NAME).yaml test/config/custom-policies/$(NAME).yaml
