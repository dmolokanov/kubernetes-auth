#!/bin/sh
set -e

# read service account token from filesystem
file="/var/run/secrets/kubernetes.io/serviceaccount/token"
if [[ -e ${file} ]]
then
    token=`cat ${file}`
else
    echo "Could not find file ${file}"
fi

config_src="/etc/traefik/traefik.toml"
if [[ ! -e ${config_src} ]]
then
    echo "Could not find config ${config_src}"
    exit 1
fi

# move traefic config to a new place to allow modification
config_dst="/traefik.toml"
cp ${config_src} ${config_dst}

# find TOKEN placeholders and replace them with a token value
if [[ ! -z ${token} ]]
then
    sed -i -e 's/%TOKEN%/'${token}'/g' ${config_dst}
fi

# call traefik entry point script with a new config
/entrypoint.sh -c /traefik.toml