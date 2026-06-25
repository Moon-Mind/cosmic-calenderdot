name := 'cosmic-calenderdot'
appid := 'com.cosmic.calenderdot'
rootdir := env('DESTDIR', '')
prefix := env('PREFIX', '/usr')

# Installation paths
base-dir := absolute_path(clean(rootdir / prefix))
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')
metainfo-dst := base-dir / 'share' / 'metainfo' / appid + '.metainfo.xml'
bin-dst := base-dir / 'bin' / name
desktop-dst := base-dir / 'share' / 'applications' / appid + '.desktop'
icon-dst := base-dir / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / appid + '.svg'

# Compile release build
build *args:
    cargo build --release {{args}}

# Compile debug build
build-debug *args:
    cargo build {{args}}

# Compile release build (alias)
build-release: build

# Compile release with vendored deps
build-vendored *args: vendor-extract (build '--frozen --offline' args)

# Run clippy
check *args:
    cargo clippy --all-features {{args}} -- -W clippy::pedantic

check-json: (check '--message-format=json')

# Clean build artifacts
clean:
    cargo clean

clean-vendor:
    rm -rf .cargo vendor vendor.tar

clean-dist: clean clean-vendor

# Default: build
default: build

# Run the app
run *args:
    env RUST_BACKTRACE=full cargo run --release {{args}}

# Install files
install:
    install -Dm0755 {{ cargo-target-dir / 'release' / name }} {{bin-dst}}
    install -Dm0644 resources/app.desktop {{desktop-dst}}
    install -Dm0644 resources/app.metainfo.xml {{metainfo-dst}}
    install -Dm0644 resources/icon.svg {{icon-dst}}

# Uninstall files
uninstall:
    rm -f {{bin-dst}} {{desktop-dst}} {{icon-dst}}

# Vendor dependencies
vendor:
    mkdir -p .cargo
    cargo vendor --sync Cargo.toml | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    echo >> .cargo/config.toml
    rm -rf .cargo vendor

vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar

# Bump version, commit, tag
tag version:
    find -type f -name Cargo.toml -exec sed -i '0,/^version/s/^version.*/version = "{{version}}"/' '{}' \; -exec git add '{}' \;
    cargo check
    cargo clean
    git add Cargo.lock
    git commit -m 'release: {{version}}'
    git tag -a {{version}} -m ''
