.PHONY: build clean test

LIB_DIR := $(CURDIR)
TARGET_DIR := $(LIB_DIR)/../../target/release

build:
	cd $(LIB_DIR)/../.. && cargo build --release -p alpha-ta-go

test: build
	cd $(LIB_DIR)/go && go test -v ./ta

clean:
	rm -rf $(TARGET_DIR)/libalpha_ta_go.*
	rm -rf $(LIB_DIR)/go/ta/*.test
