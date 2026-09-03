.PHONY: build clean test

LIB_DIR := $(CURDIR)
TARGET_DIR := $(LIB_DIR)/../../target/release

build:
	cd $(LIB_DIR)/../.. && cargo build --release --locked -p finkit-go

test: build
	cd $(LIB_DIR)/go && LD_LIBRARY_PATH="$(TARGET_DIR):$${LD_LIBRARY_PATH:-}" go test -v ./...

clean:
	rm -rf $(TARGET_DIR)/libfinkit_go.*
	rm -rf $(LIB_DIR)/go/ta/*.test
