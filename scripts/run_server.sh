#!/bin/bash

setcap 'cap_net_admin=eip'  ./wontun

./wontun --conf tun0.conf &
pid=$!

ip addr add 172.16.0.1/24 dev tun0
ip link set up dev tun0
ip link set dev tun0 mtu 1400

nc -l 172.16.0.1 8080 &
ncpid=$!

trap "kill $pid $ncpid" INT TERM

wait $pid
wait $ncpid
