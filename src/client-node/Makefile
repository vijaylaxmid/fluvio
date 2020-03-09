DYLIB = ../../target/debug/libflv_node_client.dylib

all:	build

build:	
	nj-cli build

build-local:	
	npm link

test:	build
	node test.js

pack:
	npm	pack
	mv fluvio-client* /tmp


publish:
	npm publish --access public


clean:
	rm -rf dylib
