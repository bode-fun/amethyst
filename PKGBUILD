# Maintainer: Matt C     <matt[at]tar[dot]black>
# Developer:  axtlos   <axtlos[at]tar[dot]black>
# Developer:  Michal S <michal[at]tar[dot]black>

pkgname=amethyst
pkgver=3.3.0
pkgrel=2
pkgdesc="A fast and efficient AUR helper"
arch=('x86_64')
url="https://github.com/crystal-linux/amethyst"
license=('GPL3')
source=("git+$url")
sha256sums=('SKIP')
depends=('git' 'binutils' 'fakeroot' 'pacman-contrib' 'vim')
makedepends=('cargo')
conflicts=('ame')

prepare() {
    cd "$srcdir/$pkgname"
    cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
    cd "$srcdir/$pkgname"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release --all-features
}

package() {
    cd "$srcdir/$pkgname"
    find target/release \
        -maxdepth 1 \
        -executable \
        -type f \
        -exec install -Dm0755 -t "${pkgdir}/usr/bin/" {} +
}
