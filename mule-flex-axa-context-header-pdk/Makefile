TARGET		:= wasm32-wasi
TARGET_DIR	:= target/$(TARGET)/release
NAME		:= axa_context_header
POLICY_NAME	:= Axa Context Header
VERSION		:= 1.0.3
ASSETID		:= ${ANYPOINT_ORG_ID}/axa-context-header/$(VERSION)
ASSETIMPLID	:= ${ANYPOINT_ORG_ID}/axa-context-header-impl/$(VERSION)

.phony: build
build: manifest.yaml
	@cargo build --target $(TARGET) --release
	@cat $(TARGET_DIR)/$(NAME).wasm | base64 | tr -d '\n\r' > $(TARGET_DIR)/$(NAME).b64
	@awk 'BEGIN{getline l < "$(TARGET_DIR)/$(NAME).b64"}/<ENCODED>/{gsub("<ENCODED>",l)}1' manifest.yaml > $(TARGET_DIR)/$(NAME).yaml

.phony: deploy
deploy: build
	cp $(TARGET_DIR)/$(NAME).yaml test/config/custom-policies/$(NAME).yaml

.phony: run
run: deploy
	docker compose -f test/docker-compose.yaml up

.phony: clean
clean:
	@cargo clean -p $(NAME)
	@docker compose -f test/docker-compose.yaml rm -f

.phony: publish
publish: build
	@anypoint-cli-v4 exchange:asset:upload $(ASSETID) --type policy --name "$(POLICY_NAME)" --files='{"schema.json":"policy-schema.json","metadata.yaml":"policy-metadata.yaml"}' --status published
	@anypoint-cli-v4 exchange:asset:upload $(ASSETIMPLID) --type policy-implementation  --name "$(POLICY_NAME)" --files='{"binary.wasm":"$(TARGET_DIR)/$(NAME).wasm","metadata.yaml":"policy-impl-metadata.yaml"}' --dependencies "$(subst /,:,$(ASSETID))" --status published
