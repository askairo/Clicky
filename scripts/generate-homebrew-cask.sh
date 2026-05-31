#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 6 ]]; then
  echo "Usage: $0 <version> <sha256> <url> <homepage> <desc> <app_name> [cask_token]" >&2
  exit 1
fi

version="$1"
sha256="$2"
url="$3"
homepage="$4"
desc="$5"
app_name="$6"
cask_token="${7:-clicky}"

cat <<EOF
cask "${cask_token}" do
  version "${version}"
  sha256 "${sha256}"

  url "${url}"
  name "${app_name}"
  desc "${desc}"
  homepage "${homepage}"

  app "${app_name}.app"
end
EOF
