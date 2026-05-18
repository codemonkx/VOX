.PHONY: all build install clean

all: build

build:
	cargo build --release

install: build
	@mkdir -p $(HOME)/.local/bin
	@ln -sf $(CURDIR)/target/release/vox $(HOME)/.local/bin/vox
	@echo "Installed to ~/.local/bin/vox"
	@echo "Run 'vox' to launch."

clean:
	cargo clean

uninstall:
	@rm -f $(HOME)/.local/bin/vox
	@echo "Uninstalled."
