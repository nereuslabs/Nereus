class Nereus < Formula
  desc "Terminal-First AI Programming Mentor Built in Rust"
  homepage "https://nereuslabs.github.io"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nereuslabs/nereus/releases/download/v0.2.0/nereus-v0.2.0-darwin-arm64.tar.gz"
      sha256 "88bb9bd0ad9b90af1cf3d80e51ae6cb58816fb57aff6518dcfc7cf8ecd6e1dd2"
    else
      url "https://github.com/nereuslabs/nereus/releases/download/v0.2.0/nereus-v0.2.0-darwin-x64.tar.gz"
      sha256 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/nereuslabs/nereus/releases/download/v0.2.0/nereus-v0.2.0-linux-arm64.tar.gz"
      sha256 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    else
      url "https://github.com/nereuslabs/nereus/releases/download/v0.2.0/nereus-v0.2.0-linux-x64.tar.gz"
      sha256 "930da8904947cf2350058439b563ed4fbbc2174aab72e1a39e44028434a133bb"
    end
  end

  def install
    bin.install "nereus"
  end

  test do
    assert_match "nereus", shell_output("#{bin}/nereus --version")
  end
end
