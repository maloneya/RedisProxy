.DEFAULT_GOAL := build

build:
	docker build -t redisproxy .

test: 
	docker-compose up --build