class Nereus < Formula
  desc "Terminal-First AI Programming Mentor Built in Rust"
  homepage "https://nereuslabs.github.io"
  version "0.3.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nereuslabs/nereus/releases/download/v0.3.0/nereus-v0.3.0-darwin-arm64.tar.gz"
      sha256 "73dda425c81e5501c899ebed9974875d529f77dfe80e77601e0dd4e559defc62"
    else
      url "https://github.com/nereuslabs/nereus/releases/download/v0.3.0/nereus-v0.3.0-darwin-x64.tar.gz"
      sha256 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/nereuslabs/nereus/releases/download/v0.3.0/nereus-v0.3.0-linux-arm64.tar.gz"
      sha256 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    else
      url "https://github.com/nereuslabs/nereus/releases/download/v0.3.0/nereus-v0.3.0-linux-x64.tar.gz"
      sha256 "2806001de2d86c11ebfecdbb9b9a92d2fb98448a24d43d00f74178f067cc3c11"
    end
  end

  def install
    bin.install "nereus"
  end

  test do
    assert_match "nereus", shell_output("#{bin}/nereus --version")
  end
end
