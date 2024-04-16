#!/bin/bash
set -e
set -x

if [ -z "$1" ]; then
  read -r -p "This will delete all existing files. Continue? <ENTER>"
fi

PKI_ROOT_DIR="$(dirname "$0")/"
PKI_STORE_DIR="$(dirname "$0")/store"

rm -f "$PKI_STORE_DIR"/*.{key,pem}
rm -rf "$PKI_STORE_DIR"/deploy/

"$PKI_ROOT_DIR"/generate-ca.sh
"$PKI_ROOT_DIR"./generate-certificate.sh opendut.local
"$PKI_ROOT_DIR"./generate-certificate.sh auth.opendut.local
"$PKI_ROOT_DIR"./generate-certificate.sh netbird.opendut.local
"$PKI_ROOT_DIR"./generate-certificate.sh netbird-api.opendut.local
"$PKI_ROOT_DIR"./generate-certificate.sh signal.opendut.local
"$PKI_ROOT_DIR"./generate-certificate.sh carl.opendut.local
