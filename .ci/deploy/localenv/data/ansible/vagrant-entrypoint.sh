#!/usr/bin/env bash

cd /vagrant || { echo Could not change directory to /vagrant; exit 1; }
docker compose --file .ci/deploy/localenv/docker-compose.yml build
docker compose --file .ci/deploy/localenv/docker-compose.yml up --detach

echo "All containers started. You may observe the containers by connecting to the VM:"
echo "vagrant ssh"

echo "The following secrets were created:"
cat .ci/deploy/localenv/.env

echo -e "\n---------------------\n"
echo "docker ps"
echo "cd /vagrant"
echo "docker compose logs --tail=0 --follow"
