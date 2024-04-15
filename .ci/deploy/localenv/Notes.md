
### Test connection with edgar in opendut-vm

```shell

# ssh into the vm
cargo theo vagrant ssh

# start the edgar service
cargo theo dev edgar-shell
# or manually
OPENDUT_EDGAR_REPLICAS=3 docker compose -f .ci/docker/edgar/docker-compose.yml run --entrypoint="" peer bash

# ping target carl address and check if the connection is successful (modify /etc/hosts if necessary)
apt-get install nano && nano /etc/hosts

# remove all environment variables that are preset in test environment and should not be used for the test
env -i bash
export OPENDUT_EDGAR_SERVICE_USER=root
tar xf artifacts/opendut-edgar-x86_64-unknown-linux-gnu-*
tar xf artifacts/opendut-cleo-x86_64-unknown-linux-gnu-*

# setup the peer
/opt/opendut-edgar/opendut-edgar setup managed

```


```shell
# cleo env vars
OPENDUT_CLEO_NETWORK_CARL_HOST=carl.opendut.local
OPENDUT_CLEO_NETWORK_CARL_PORT=443
OPENDUT_CLEO_NETWORK_OIDC_ENABLED=true
OPENDUT_CLEO_NETWORK_OIDC_CLIENT_CLIENT_ID=opendut-cleo-client
OPENDUT_CLEO_NETWORK_OIDC_CLIENT_CLIENT_SECRET=918642e0-4ec4-4ef5-8ae0-ba92de7da3f9
OPENDUT_CLEO_NETWORK_OIDC_CLIENT_ISSUER_URL=https://auth.opendut.local/realms/opendut/
OPENDUT_CLEO_NETWORK_OIDC_CLIENT_SCOPES=

```
