FROM ubuntu:22.04

RUN apt update

RUN apt install -y iproute2 libcap2-bin netcat

COPY target/release/wontun /wontun 

COPY scripts/run_server.sh /run_server.sh

COPY tun0-server.conf /tun0.conf

CMD bash run_server.sh
