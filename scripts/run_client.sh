#!/bin/bash


IP=$(target/release/wontun-conf --conf tun0.conf | jq -r '"\(.interface.address[0])/\(.interface.address[1])"')

sudo setcap cap_net_admin=eip target/release/wontun
target/release/wontun --conf tun0.conf &
pid=$!

sudo ip addr add $IP dev tun0
sudo ip link set up dev tun0
sudo ip link set dev tun0 mtu 1400

trap "kill $pid" INT TERM
wait $pid
