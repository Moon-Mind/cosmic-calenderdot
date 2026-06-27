name := 'cosmic-calenderdot'
appid := 'com.cosmic.calenderdot'
rootdir := env('DESTDIR', '')
prefix := env('PREFIX', '/usr')

base-dir := absolute_path(clean(rootdir / prefix))
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')
metainfo-dst := base-dir / 'share' / 'metainfo' / appid + '.metainfo.xml'
bin-dst := base-dir / 'bin' / name
desktop-dst := base-dir / 'share' / 'applications' / appid + '.desktop'
icon-dst := base-dir / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / appid + '.svg'

default: build

build:
    cargo build --release

install:
    install -Dm0755 {{ cargo-target-dir / 'release' / name }} {{bin-dst}}
    install -Dm0644 resources/app.desktop {{desktop-dst}}
    install -Dm0644 resources/app.metainfo.xml {{metainfo-dst}}
    install -Dm0644 resources/icon.svg {{icon-dst}}

uninstall:
    rm -f {{bin-dst}} {{desktop-dst}} {{icon-dst}}

clean:
    cargo clean

run:
    RUST_BACKTRACE=full cargo run --release
