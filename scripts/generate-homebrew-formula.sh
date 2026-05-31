#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 5 ]]; then
  echo "Usage: $0 <version> <sha256> <url> <homepage> <desc> [formula_name] [app_name]" >&2
  exit 1
fi

version="$1"
sha256="$2"
url="$3"
homepage="$4"
desc="$5"
formula_name="${6:-clicky}"
app_name="${7:-Clicky}"
formula_class="$(echo "${formula_name}" | awk '{print toupper(substr($0,1,1)) substr($0,2)}')"

cat <<EOF
class ${formula_class} < Formula
  desc "${desc}"
  homepage "${homepage}"
  url "${url}"
  version "${version}"
  sha256 "${sha256}"

  def install
    prefix.install Dir["*"]
  end

  def caveats
    <<~EOS
      ${app_name} is a GUI app package.
      Extracted files are installed under:
        \#{opt_prefix}
    EOS
  end
end
EOF
