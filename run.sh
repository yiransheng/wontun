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

# Check if an argument is provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <client|server>"
    exit 1
fi

# Run based on the provided argument
case "$1" in
    client)
        echo "run client"
        ./scripts/run_client.sh
        ;;
    server)
        make wontun-remote-docker

        # Set trap for INT and TERM signals
        trap cleanup INT TERM

        echo "run server"
        # Run the Docker container in the background
        docker run --name wontun-remote \
          --rm --network=wontun-test --cap-add=NET_ADMIN \
          --device=/dev/net/tun wontun-remote:latest &

        # Wait for the Docker container process to exit
        wait $!
        ;;
    *)
        echo "Invalid argument: $1"
        echo "Usage: $0 <client|server>"
        exit 1
        ;;
esac

