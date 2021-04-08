pkgname=myxer
_pkgname=Myxer
pkgver=1.2.0
pkgrel=1
pkgdesc='A modern Volume Mixer for PulseAudio, built with you in mind.'
url='https://github.com/Aurailus/Myxer'
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/$pkgver.tar.gz")
arch=('any')
license=('GPL3')
makedepends=('cargo')
depends=('pulseaudio' 'gtk3')
sha256sums=('4784746fd491d51397b3c47eb5ed5cf3f04ba54a116c192620bb532db2c2d550')

build () {
  cd "$srcdir/$_pkgname-$pkgver"

  cargo build --release
}

package() {
  cd "$srcdir/$_pkgname-$pkgver"

  install -Dm755 target/release/$pkgname "${pkgdir}/usr/bin/myxer"
}
