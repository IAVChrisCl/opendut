FROM ubuntu:22.04

RUN apt-get update && apt-get dist-upgrade -y && apt-get install -y nginx-extras apache2-utils

COPY ./.ci/docker/nginx-webdav/webdav.conf /etc/nginx/conf.d/default.conf
RUN rm -rf /etc/nginx/sites-enabled/*

RUN mkdir -p "/media/data"

RUN chown -R www-data:www-data "/media/data"

VOLUME /media/data

COPY ./.ci/docker/nginx-webdav/entrypoint.sh /
RUN chmod +x entrypoint.sh
CMD /entrypoint.sh && nginx -g "daemon off;"