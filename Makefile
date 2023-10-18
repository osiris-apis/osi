#
# Maintenance Makefile
#

# Enforce bash with fatal errors.
SHELL			:= /bin/bash -eo pipefail

# Keep intermediates around on failures for better caching.
.SECONDARY:

# Default build and source directories.
BUILDDIR		?= ./build
SRCDIR			?= .

#
# Build Images
#

# https://github.com/osiris-apis/plumbing/pkgs/container/osiris-ci
IMG_CI			?= ghcr.io/osiris-apis/osiris-ci:latest

#
# Common Commands
#

DOCKER_RUN		= \
	docker \
		run \
		--interactive \
		--rm

DOCKER_RUN_SELF		= \
	$(DOCKER_RUN) \
		--user "$$(id -u):$$(id -g)"

#
# Target: help
#

.PHONY: help
help:
	@# 80-width marker:
	@#     01234567012345670123456701234567012345670123456701234567012345670123456701234567
	@echo "make [TARGETS...]"
	@echo
	@echo "The following targets are provided by this maintenance makefile:"
	@echo
	@echo "    help:               Print this usage information"
	@echo
	@echo "    rust-build:         Build the Rust packages"
	@echo "    rust-test:          Run the Rust test suite"

#
# Target: BUILDDIR
#

$(BUILDDIR)/:
	mkdir -p "$@"

$(BUILDDIR)/%/:
	mkdir -p "$@"

#
# Target: FORCE
#
# Used as alternative to `.PHONY` if the target is not fixed.
#

.PHONY: FORCE
FORCE:

#
# Target: rust-*
#

RUST_CHANNEL		?= stable

.PHONY: rust-build
rust-build: $(BUILDDIR)/rust/ $(BUILDDIR)/cargo/
	$(DOCKER_RUN_SELF) \
		--env "CARGO_HOME=/srv/build/cargo" \
		--init \
		--volume "$(abspath $(BUILDDIR)):/srv/build" \
		--volume "$(abspath $(SRCDIR)):/srv/src" \
		--workdir "/srv/src" \
		"$(IMG_CI)" \
			cargo \
				"+$(RUST_CHANNEL)" \
				build \
				--all-targets \
				--target-dir "/srv/build/rust" \
				--verbose \
				--workspace

.PHONY: rust-test
rust-test: $(BUILDDIR)/rust/
	$(DOCKER_RUN_SELF) \
		--env "CARGO_HOME=/srv/build/cargo" \
		--init \
		--volume "$(abspath $(BUILDDIR)):/srv/build" \
		--volume "$(abspath $(SRCDIR)):/srv/src" \
		--workdir "/srv/src" \
		"$(IMG_CI)" \
			cargo \
				"+$(RUST_CHANNEL)" \
				test \
				--all-targets \
				--target-dir "/srv/build/rust" \
				--verbose \
				--workspace
