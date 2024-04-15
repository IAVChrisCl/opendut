#!/bin/bash

create_password() {
  length="${1:-32}"
  #password="$(openssl rand -base64 "$length")"
  # pwgen produces more readable passwords
  password="$(pwgen "$length")"
  echo "$password"
}

# Keycloak
echo POSTGRES_PASSWORD="$(create_password)"
echo KEYCLOAK_ADMIN_PASSWORD="$(create_password)"

# Netbird
echo NETBIRD_DATASTORE_ENC_KEY="$(create_password)"
echo TURN_PASSWORD="$(create_password)"
echo NETBIRD_MANAGEMENT_CLIENT_SECRET="$(create_password)"
echo NETBIRD_PASSWORD="$(create_password 16)"

# CARL / Keycloak
echo OPENDUT_CARL_NETWORK_OIDC_CLIENT_CLIENT_SECRET="$(create_password)"
echo OPENDUT_CARL_NETWORK_OIDC_CLIENT_PEER_SECRET="$(create_password)"
echo OPENDUT_EDGAR_NETWORK_OIDC_CLIENT_CLIENT_SECRET="$(create_password)"
echo OPENDUT_CLEO_NETWORK_OIDC_CLIENT_CLIENT_SECRET="$(create_password)"
echo OPENDUT_USER_OPENDUT="$(create_password 16)"
