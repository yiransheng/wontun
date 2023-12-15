#!/bin/bash

IP=$(target/release/wontun-conf --conf tun0.conf | jq -r '"\(.interface.address[0])/\(.interface.address[1])"')

setcap cap_net_admin=eip target/release/wontun
target/release/wontun --conf tun0.conf --log-level debug &
pid=$!

set -x

ip addr add $IP dev tun0
ip link set up dev tun0
ip link set dev tun0 mtu 1400

# ip -4 route add 0.0.0.0/0 dev tun0 table 19988
# ip -4 rule add not fwmark 19988 table 19988
# ip -4 rule add table main suppress_prefixlength 0
# resolvectl dns tun0 1.1.1.1

cleanup() {
  kill $pid;
  # ip -4 rule delete table 19988
  # ip -4 rule delete table main suppress_prefixlength 0
  # iptables-restore -n
}

trap "cleanup" INT TERM
wait $pid
