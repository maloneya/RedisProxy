from redis import Redis
import requests
from multiprocessing import Pool
import string
import random


def generate_test_data(size):
    test_data = {}
    for i in range(size):
        letters = string.ascii_lowercase
        random_key = "".join(random.choice(letters) for i in range(10))
        random_val = "".join(random.choice(letters) for i in range(10))
        test_data[random_key] = random_val

    return test_data


def set_in_redis(test_data):
    r = Redis(host="localhost", port=6379, db=0)
    for k, v in test_data.items():
        r.set(k, v)


def get_from_proxy(test_data):
    (k, v) = test_data
    request = f"http://localhost:8000/get?key={k}"
    proxy_res = requests.get(request)
    print(proxy_res.text)
    assert proxy_res.text == v


test_data = generate_test_data(40)
set_in_redis(test_data)

with Pool(10) as p:
    p.map(get_from_proxy, test_data.items())
