FROM debian:12.5-slim

COPY target/release/v6tprouter /v6tprouter

RUN apt-get update &&\
    apt-get -y install --no-install-recommends --no-install-suggests iproute2 radvd ndppd &&\
    apt-get -y clean && rm -rf /var/lib/apt/lists/* &&\
	chmod +x /v6tprouter

CMD ["/v6tprouter"]
