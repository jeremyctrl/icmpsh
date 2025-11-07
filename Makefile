all: client server

client/client.exe: client/client.c
	clang client/client.c \
		--target=x86_64-w64-mingw32 \
		-I/usr/x86_64-w64-mingw32/include \
		-L/usr/x86_64-w64-mingw32/lib \
		-liphlpapi -lws2_32 \
		-o client/client.exe

client: client/client.exe

server/target/release/server: $(wildcard server/src/*.rs) server/Cargo.toml
	cd server && cargo build --release

server: server/target/release/server

clean:
	rm -f client/client.exe
	cd server && cargo clean