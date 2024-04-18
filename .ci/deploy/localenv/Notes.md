# Notes

Notes on how to connect to the local environment.

## Test connection with edgar in opendut-vm

```shell

# ssh into the vm
cargo theo vagrant ssh

# make sure can modules are loaded
modprobe can-gw vcan

# start the edgar service
cargo theo dev edgar-shell
# or manually
OPENDUT_EDGAR_REPLICAS=3 docker compose -f .ci/docker/edgar/docker-compose.yml run --entrypoint="" peer bash

# start container with different IP address (does not use opendut network bridge)
OPENDUT_EDGAR_REPLICAS=3 docker compose -f .ci/docker/edgar/docker-compose-edgar-test.yml run --entrypoint="" edgar bash

# ping target carl address and check if the connection is successful (modify /etc/hosts if necessary)
apt-get install nano && nano /etc/hosts
# insert the following
192.168.56.9 opendut.local
192.168.56.9 auth.opendut.local
192.168.56.9 netbird.opendut.local
192.168.56.9 netbird-api.opendut.local
192.168.56.9 signal.opendut.local
192.168.56.9 carl.opendut.local
# ping should work
ping carl.opendut.local


# remove all environment variables that are preset in test environment and should not be used for the test
env -i bash
export OPENDUT_EDGAR_SERVICE_USER=root
tar xf artifacts/opendut-edgar-x86_64-unknown-linux-gnu-*
tar xf artifacts/opendut-cleo-x86_64-unknown-linux-gnu-*

# setup the peer
/opt/opendut-edgar/opendut-edgar setup managed

```

## CURL

Setting the `SSL_CERT_FILE` environment variable is necessary to connect to the local environment with curl.
```
export OPENDUT_REPO_ROOT=$(git rev-parse --show-toplevel)
export SSL_CERT_FILE=${OPENDUT_REPO_ROOT:-.}/.ci/deploy/localenv/data/pki/store/insecure-development-ca.pem
curl https://carl.opendut.local

```

## CLEO

* Decode peer setup
```
cargo ci cleo run -- decode-peer-setup
```

* Configure instance

Either create toml file here `~/.config/opendut/cleo/config.toml`

or use environment variables

```shell
# cleo env vars
export OPENDUT_CLEO_NETWORK_CARL_HOST=carl.opendut.local
export OPENDUT_CLEO_NETWORK_TLS_DOMAIN_NAME_OVERRIDE=carl.opendut.local
export OPENDUT_CLEO_NETWORK_CARL_PORT=443
export OPENDUT_CLEO_NETWORK_TLS_CA=/etc/opendut/tls/ca.pem
export OPENDUT_CLEO_NETWORK_OIDC_ENABLED=true
export OPENDUT_CLEO_NETWORK_OIDC_CLIENT_CLIENT_ID=opendut-cleo-client
export OPENDUT_CLEO_NETWORK_OIDC_CLIENT_CLIENT_SECRET=918642e0-4ec4-4ef5-8ae0-ba92de7da3f9
export OPENDUT_CLEO_NETWORK_OIDC_CLIENT_ISSUER_URL=https://auth.opendut.local/realms/opendut/
export OPENDUT_CLEO_NETWORK_OIDC_CLIENT_SCOPES=

export OPENDUT_REPO_ROOT=$(git rev-parse --show-toplevel)
export SSL_CERT_FILE=${OPENDUT_REPO_ROOT:-.}/.ci/deploy/localenv/data/pki/store/insecure-development-ca.pem
export OPENDUT_CLEO_NETWORK_TLS_CA=$SSL_CERT_FILE

```
