RUSTC ?= rustc
RUSTDOC ?= rustdoc
RUSTC_FLAGS ?= -O

SRC = $(shell find src -name '*.rs')

all: spread

spread: $(SRC)
	mkdir -p target
	$(RUSTC) --out-dir target src/lib.rs

test: $(SRC)
	mkdir -p target
	$(RUSTC) --test --out-dir target src/lib.rs
	./target/spread

.PHONY: doc
doc: $(SRC)
	$(RUSTDOC) $<

.PHONY: clean
clean:
	@rm -rf target
