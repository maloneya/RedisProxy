.DEFAULT_GOAL := build

build:
	sudo docker build -t redisproxy .

run:
	# tood run script to set values in redis
	# fix sudo 
	sudo docker run --name backing-redis -d redis
	sudo docker run -it --rm --name app redisproxy

test: 
	@echo "todo" 

clean:
	@echo "todo" 