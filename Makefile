build:
	wasm-pack build --target web

serve:
	python -m http.server 4242

run:
	x-www-browser http://localhost:4242
