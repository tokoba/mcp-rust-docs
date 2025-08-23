install:
	cargo install cargo-license cargo-about

build:
	cargo build --release
	cargo about generate about.hbs > ./license.html
