#!/bin/bash

sudo setcap cap_net_admin=eip target/release/wontun
target/release/wontun --conf tun0.conf &
pid=$!

sudo ip addr add 172.16.0.3/24 dev tun0
sudo ip link set up dev tun0
sudo ip link set dev tun0 mtu 1400

trap "kill $pid" INT TERM
wait $pid
