class Mcpforge < Formula
  desc "The fast, zero-runtime TUI that discovers every MCP client on your machine and syncs them all"
  homepage "https://github.com/nordicnode/mcpforge"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nordicnode/mcpforge/releases/download/v0.1.0/mcpforge-aarch64-apple-darwin.tar.gz"
    else
      url "https://github.com/nordicnode/mcpforge/releases/download/v0.1.0/mcpforge-x86_64-apple-darwin.tar.gz"
    end
  end

  on_linux do
    url "https://github.com/nordicnode/mcpforge/releases/download/v0.1.0/mcpforge-x86_64-unknown-linux-gnu.tar.gz"
  end

  def install
    bin.install "mcpforge"
  end

  test do
    assert_match "mcpforge", shell_output("#{bin}/mcpforge --version")
  end
end
