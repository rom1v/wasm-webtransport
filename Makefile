export RUSTFLAGS=--cfg=web_sys_unstable_apis

build:
	wasm-pack build --target web

serve:
	python -m http.server 4242

run:
	chromium \
		--origin-to-force-quic-on=localhost:4433 \
		--ignore-certificate-errors-spki-list=rOxva4Y8FcAUzOje9N66vJTYLxhSK9r5t2tVVEe2bdE= \
		http://localhost:4242/client.html
