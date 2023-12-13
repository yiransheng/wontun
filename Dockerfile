FROM ubuntu:22.04

RUN apt update

RUN apt install -y iproute2 libcap2-bin netcat jq

COPY target/release/wontun /wontun 
COPY target/release/wontun-conf /wontun-conf

COPY scripts/run_docker.sh /run_docker.sh

COPY *.conf / 

CMD bash run_docker.sh
