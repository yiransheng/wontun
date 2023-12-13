#!/bin/bash

cargo b --release
ext=$?
if [[ $ext -ne 0 ]]; then
    exit $ext
fi

cleanup() {
    echo "Signal caught, killing the Docker container..."
    docker kill wontun-remote
    exit 0
}

# Check if at least one argument is provided
if [ $# -lt 1 ]; then
    echo "Usage: $0 host | docker <conf>"
    exit 1
fi

# Handle the first argument
case "$1" in
    host)
        # Execute the host script
        ./scripts/run_host.sh
        ;;
    docker)
        # Check if the 'conf' argument is provided
        if [ $# -eq 2 ]; then
            make wontun-remote-docker

            # Set trap for INT and TERM signals
            trap cleanup INT TERM

            echo "run docker"
            # Run the Docker container in the background
            docker run --name wontun-remote \
                --env WONTUN_CONF=$2 \
                --rm \
                --network=wontun-test \
                --cap-add=NET_ADMIN \
                --device=/dev/net/tun \
                wontun-remote:latest &

            # Wait for the Docker container process to exit
            wait $!
        else
            echo "Error: 'conf' argument is required for docker"
            exit 1
        fi
        ;;
    *)
        echo "Invalid argument: $1"
        echo "Usage: $0 host | docker <conf>"
        exit 1
        ;;
esac

