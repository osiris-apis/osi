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

.PHONY: rust-builddir
rust-builddir: $(BUILDDIR)/cargo/ $(BUILDDIR)/rust/

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
				--verbose \
				--workspace

.PHONY: rust-doc
rust-doc: rust-builddir
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
				--workspace
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
				--verbose \
				--workspace
