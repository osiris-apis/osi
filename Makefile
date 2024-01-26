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

# Credentials of the caller.
UID			:= $(shell id -u)
GID			:= $(shell id -g)

#
# Build Images
#

# https://github.com/osiris-apis/plumbing/pkgs/container/osiris-ci
IMG_CI			?= ghcr.io/osiris-apis/osiris-ci:latest
# https://quay.io/repository/podman/stable
IMG_PODMAN		?= quay.io/podman/stable:latest

#
# Common Commands
#

# Run one-shot container.
DOCKER_RUN		= \
	docker \
		run \
		--interactive \
		--rm

# Run one-shot privileged container.
DOCKER_RUN_PRIV		= \
	$(DOCKER_RUN) \
		--privileged \
		--volume "/var/lib/containers:/var/lib/containers" \
		--volume "/var/run/docker.sock:/var/run/docker.sock" \
		--volume "/:/host"

# Run one-shot container with uid/gid mapped to 1000/1000. The container is run
# in a nested and privileged podman instance. Yet, the container itself does
# not run in privileged mode.
# The nesting allows running newer podman versions than available on the host.
DOCKER_PRIV_PODMAN_RUN_1000	= \
	$(DOCKER_RUN_PRIV) \
		$(IMG_PODMAN) \
		podman \
			run \
			--interactive \
			--rm \
			--gidmap "0:0:1000" \
			--gidmap "+1000:$(GID):1" \
			--uidmap "0:0:1000" \
			--uidmap "+1000:$(UID):1" \
			--user "1000:1000"

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
	@echo "    rust-doc:           Build the Rust documentation"
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

# Rust channel to use for all operations.
RUST_CHANNEL		?= stable

# List of Rust targets this workspace has crates for.
RUST_TARGETS		?= \
	x86_64-apple-darwin \
	x86_64-linux-android \
	x86_64-pc-windows-msvc \
	x86_64-unknown-linux-gnu

rust-builddir-%: | $(BUILDDIR)/rust/%/
	@ln -fns "../doc" "$(BUILDDIR)/rust/$*/doc"

.PHONY: rust-builddir
rust-builddir: \
    $(foreach target,$(RUST_TARGETS),rust-builddir-$(target)) \
    | \
    $(BUILDDIR)/cargo/ \
    $(BUILDDIR)/rust/ \
    $(BUILDDIR)/rust/doc/ \

.PHONY: rust-build
rust-build: rust-builddir
	$(DOCKER_PRIV_PODMAN_RUN_1000) \
		--env "CARGO_HOME=/srv/build/cargo" \
		--init \
		--volume "/host/$(abspath $(BUILDDIR)):/srv/build" \
		--volume "/host/$(abspath $(SRCDIR)):/srv/src" \
		--workdir "/srv/src" \
		"$(IMG_CI)" \
			cargo \
				"+$(RUST_CHANNEL)" \
				build \
				--all-targets \
				--target-dir "/srv/build/rust" \
				--verbose

F_RUST_DOC		= \
	$(DOCKER_PRIV_PODMAN_RUN_1000) \
		--env "CARGO_HOME=/srv/build/cargo" \
		--init \
		--volume "/host/$(abspath $(BUILDDIR)):/srv/build" \
		--volume "/host/$(abspath $(SRCDIR)):/srv/src" \
		--workdir "/srv/src" \
		"$(IMG_CI)" \
			cargo \
				"+$(RUST_CHANNEL)" \
				doc \
				--lib \
				--no-deps \
				--target-dir "/srv/build/rust" \
				--verbose \
				$1

RUST_DOC_PKGS		=

rust-doc-%: rust-builddir FORCE
	test -z "$(RUST_DOC_PKGS)" || \
		$(call \
			F_RUST_DOC, \
			--target "$*" \
			$(foreach pkg,$(RUST_DOC_PKGS),-p $(pkg)) \
		)

rust-doc-x86_64-apple-darwin: RUST_DOC_PKGS = osiris-macos
rust-doc-x86_64-pc-windows-msvc: RUST_DOC_PKGS = osiris-windows
rust-doc-x86_64-unknown-linux-gnu: RUST_DOC_PKGS = osiris-linux

.PHONY: rust-doc
rust-doc: $(foreach target,$(RUST_TARGETS),rust-doc-$(target))
	$(call F_RUST_DOC,)
	rm -f "$(BUILDDIR)/rust/doc/.lock"

.PHONY: rust-test
rust-test: rust-builddir
	$(DOCKER_PRIV_PODMAN_RUN_1000) \
		--env "CARGO_HOME=/srv/build/cargo" \
		--init \
		--volume "/host/$(abspath $(BUILDDIR)):/srv/build" \
		--volume "/host/$(abspath $(SRCDIR)):/srv/src" \
		--workdir "/srv/src" \
		"$(IMG_CI)" \
			cargo \
				"+$(RUST_CHANNEL)" \
				test \
				--all-targets \
				--target-dir "/srv/build/rust" \
				--verbose
