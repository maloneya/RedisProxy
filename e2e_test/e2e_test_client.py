from redis import Redis
import requests
from multiprocessing import Pool
import string
import random
import time
import sys

"""
Usage 
python3 e2e_test_client.py [proxy net] [proxy port] [redis net]

arguments are required

i.e.
python3 e2e_test_client.py proxy 8000 redis
"""

# let the proxy and redis start
# a better approach here would be to expose a 'ready' signal from the proxy
time.sleep(3)

assert len(sys.argv) == 4, "incorrect arguments"
PROXY_NETWORK = sys.argv[1]
PROXY_PORT = sys.argv[2]
REDIS_NETWORK = sys.argv[3]


def generate_test_data(size):
    test_data = {}
    for i in range(size):
        letters = string.ascii_lowercase
        random_key = "".join(random.choice(letters) for i in range(10))
        random_val = "".join(random.choice(letters) for i in range(10))
        test_data[random_key] = random_val

    return test_data


def set_in_redis(test_data):
    r = Redis(host=REDIS_NETWORK, port=6379)
    for k, v in test_data.items():
        r.set(k, v)

    print(f"test client loaded {len(test_data)} random key value pairs into redis")


def get_from_proxy_expect_success(test_data):
    (k, v) = test_data
    request = f"http://{PROXY_NETWORK}:{PROXY_PORT}/{k}"
    proxy_res = requests.get(request)
    assert proxy_res.text == v


def get_from_proxy_expect_fail(test_data):
    (k, v) = test_data
    request = f"http://{PROXY_NETWORK}:{PROXY_PORT}/{k}"
    proxy_res = requests.get(request)
    assert proxy_res.text == ""


test_data = generate_test_data(1000)
set_in_redis(test_data)

bad_test_data = generate_test_data(10)

pool_size = 10
successful_requests = 0
with Pool(pool_size) as p:
    # test that the proxy can handle values missing from redis
    p.map(get_from_proxy_expect_fail, bad_test_data.items())
    successful_requests += len(bad_test_data)
    # test that the proxy can handle values not in the cache
    p.map(get_from_proxy_expect_success, test_data.items())
    successful_requests += len(test_data)
    # send same keys to test that the proxy can serve values
    # out of the cache
    p.map(get_from_proxy_expect_success, test_data.items())
    successful_requests += len(test_data)

print(
    f"test client sucessfully ran {successful_requests} get requests across {pool_size} parallel clients"
)

