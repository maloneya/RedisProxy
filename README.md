# Usage
"Single Click Build and Test" - `make test`
- This will start the backing redis, the proxy, and a test client. The test client asserts that the value it set in redis is the same value that it gets from the proxy. it also asserts that keys not set in redis do not return a result from the proxy. 
- Expected output "Test client successfully ran 2010 get requests across 10 parallel clients"
- Note: the build time for the Proxy is disappointingly slow due to all the dependencies the web framework rocket includes.

**Configuration** 
1. Address of backing redis is passed to proxy via the --redis_addr flag and set in the docker-compose.yml file
2. cache expiry time is passed to the proxy via the --cache_expr_sec flag and is currently just using the default 
3. Cache capacity is passed to the proxy via the --cache_size flag and is currently just using the default
4. TCP port the proxy listens on is configured in the DOCKERFILE for the proxy via the ROCKET_PORT env variable and is set in the docker-compose.yml file
5. the test client configures the ip of redis and the proxy as well as the proxy port. 

There are unit tests however they depend on `cargo` and the rust tool chain. They can be run via `cargo test` 

# Architecture
The proxy has is structured around a core producer consumer work queue. There are three top level components: 
1. `RedisProducer`- The RedisProducer is attached to each web worker thread and is responsible for taking incoming http requests, creating a RedisRequst and sending them to the consumer.
2. `RedisRequest` - The RedisRequest owns the key and request result. It is responsible for synchronizing the producers and consumer by providing an API for the producer to wait for a result, and for the consumer to notify the producer when the result is available. 
3. `RedisConsumer` - The RedisConsumer receives an ordered list of requests from the producers. The consumer owns a redis client and an lru cache and is responsible for orchestrating cache gets and puts and redis fetches. The RedisConsumer is loosely coupled to both the cache and the redis client, and only depending on a minimal interface for each. 

additional details can be found annotated to each struct in the implementation. 

### Test client overview 
The end to end test client is implemented in python and uses the multiprocessing library to create parallel connections to the proxy asserting the correct behavior of the 'sequential concurrent processing' requirement. The test client generates random data on each run. The test has the following structure: 
1. Generate two test data sets
2. load one data set into redis
3. test that the data set not loaded does not return results from the proxy 
4. test that the data set loaded into redis returns the expected value from the proxy 
5. send the test data a second time to (presumably) hit the proxy cache path and ensure that the expected values are returned.

The configuration used in the docker-compose setup uses 1000 test elements, but I was able to successfully test data sets of 10,000

### Design Choices 
 On language selection - The proxy is implemented in Rust. As this is a service that requires concurrent networking with local state in the LRU Cache, the memory safety of rust provides us with the confidence that concurrent requests aren't going to leak memory or create corrupted state in our cache. 

### Key assumptions 
 1. The Cache implementation can currently panic! and crash the worker thread in the case of an unexpected bug. While it is possible to catch panics from the cache in the RedisConsumer and send all requests to redis in the case of a bug in the cache, there are environments where that would be not ideal. In a distributed system where the proxy RPS and the cache hit rate is high, having 100% of requests fail the cache and go to the backing redis could take redis down with it create a cascading failure. In the case where panics from the cache are allowed to crash the proxy, the only service in our system that crashes is the proxy. 
 2. There are a few ways we could handle redis miss, we could return an HTTP 204 to indicate that the request was successful but there was no content. I chose to more closely mirror the redis protocol and return an empty string.  

### LRU Cache Design
The cache maintains two internal data structures: 
1. A hash map to provide fast get access to cache key value paris 
2. A list of cache keys ordered by last use to enable the LRU eviction policy 

Global expiry is achieved by annotating each cache entry with the put time and checking the lifetime of the entry upon each get. If the entry is found to be expired it is removed and the cache returns None.

##### Algorithmic Complexity
1. Get 
- entry not expired - O(n) - unfortunately our hash map lookup is slowed by needing to resort the lru entries
- entry expired O(n) - we need to remove the key from the LRU list. The cache code naively assumes that every key in the LRU list is in the map. 
- miss O(1) - hash map get fails and we return. 

2. Put 
- Cache has free capacity - O(1) - We just insert into both the hash map and the vec 
- Cache full - O(1) - We need to evict the oldest element first which just requires an LRU pop. 

**Possible improvements** 
- LRU list scans could be replaced with a binary search to take get from O(N) to O(log n) 
- If we handle the case where the LRU list to contain expired items we could ignore the list update on entry expiration 
- To get to O(1) on all operations I could annotate cache entries with a unique identifier and attach that to the key entry in the list of ordered elements. This would let me simply insert a new key:get_id pair to the LRU list rather then scan. When we pop the last used entry off the list we then fetch the cache entry and compare the unique identifiers to ensure that there isnt a new access token that has been added to the LRU list. 

Assumptions: 
* The same cache key will never be written twice by the consumer 
* All entries in the LRU list are contained in the cache entry hash map 

# Appendix

### Time investment 
1. Brushing up on rust - 4 ish hours - This is the first time ive written rust since my College thesis project. Ive been looking for a project to use as a chance to get back into the language and this was a perfect opportunity!
2. HTTP web service - 1 hour - With the Rocket web framework this was shockingly easy. 
3. Producer consumer work queues - 2 hours - the biggest time sink here was thinking about how to organize the project as a whole 
4. LRU cache implementation - 2 hours - took some time to research a bit and thoroughly test the code 
5. redis integration - < 1 hour 
6. writing e2e test client - 1 hour 
7. docker and docker compose - 5 hours - This is my first time using docker, understanding the peculiarities of Dockerfiles, Docker Compose and docker networking took me a bit

### To depend or not to depend 
1. HTTP server - As much as I love interacting with the Socket API, the lack of novel requirements on the HTTP layer made this an ideal candidate to simply solve with a library. My evaluation criteria was more focused on API simplicity and minimal code footprint which is what led me to select Rocket over the more widely used (and potentially more performant) actix. 
2. LRU Cache - The rust standard library does not contain an LRU. I did a brief evaluation of the most popular LRU crates on crates.io and found the most popular to be in maintenance mode. Given the lack of obvious library to use, and this projects requirements for cache entry timeout i decided to build my own. 

### Optimizations I would like to make
- The most obvious performance hit is all the string deep copies (or to use Rust's word - "Cloning"). In the RedisRequest result fetching we unfortunately have to copy the value out of the Mutex, even though that object is about to be cleaned up. This could be improved with some way to simply take the result. For large results this is particularly bad. 

### Unimplemented requirements
All of the core requirements were completed. The Bonus Requirements were not. 

I believe this architecture is well suited to achieve Parallel concurrent processing. Two key changes would need to be made. The producers and consumer currently use rusts Multi producer single consumer queue. This would need to be replaced with a multi producer multi consumer queue. Additionally the Cache implementation would need to be improved to include syncronized access to its internal data structures. This has non trivial complexity as the cache contains two collections that would need to be syncronized together. 


### References 
1. Its been awhile since I used Rust, so i referenced this book frequently https://doc.rust-lang.org/book/
2. This guide was quite useful in getting the web server up and running https://rocket.rs/v0.4/guide/requests/
3. This was also my first time using docker so i followed this https://shaneutt.com/blog/rust-fast-small-docker-image-builds/
4. For the LRU i referenced some of the ideas in this experimental std library LRU cache https://doc.rust-lang.org/0.12.0/std/collections/lru_cache/struct.LruCache.html
