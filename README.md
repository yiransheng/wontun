# `wontun` : an insecure and fragile toy vpn

## Write up

https://write.yiransheng.com/vpn

You absolutely should not use this application over the public Internet.

## Features

* Modeled after `wireguard` without the unnecessary security features and robustness
* Notably stealing a lot from [boringtun](https://github.com/cloudflare/boringtun/blob/master/boringtun/src/device/mod.rs)
* Recognizes a subset of `wireguard` configurations
* Requires a pile of bash scripts to run (best to read the article above if you want to run the thing)

## Example network topology

Terminal tab 1, using [server.conf](./server.conf):

```bash
./run.sh docker server.conf
```

Terminal tab 2, using [client-B.conf](./client-B.conf)

```bash
./run.sh docker client-B.conf
```

Terminal tab 3, using [ton0.conf](./tun0.conf)

```bash
./run.sh host
```

Terminal tab 4 (`traceroute` from host to client-B):

```bash
traceroute to 10.10.0.2 (10.10.0.2), 64 hops max
  1   10.10.0.1  0.116ms  0.179ms  0.268ms
  2   10.10.0.2  0.309ms  0.287ms  0.234ms
```

