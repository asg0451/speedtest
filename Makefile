# sudo apt install mingw-w64
release:
	cargo build --release --target x86_64-pc-windows-gnu
	cargo build --release
