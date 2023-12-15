#!/bin/bash

set -x

cp "$WONTUN_CONF" tun0.conf

IP=$(./wontun-conf --conf tun0.conf | jq -r '"\(.interface.address[0])/\(.interface.address[1])"')

setcap 'cap_net_admin=eip'  ./wontun

./wontun --conf tun0.conf --log-level debug &
pid=$!

ip addr add $IP dev tun0
ip link set up dev tun0
ip link set dev tun0 mtu 1400

trap "kill $pid $ncpid" INT TERM

wait $pid
