FROM python:3

ENV PROXY_PORT="8000"

WORKDIR /usr/src/app

COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

CMD python3 e2e_test_client.py proxy ${PROXY_PORT} redis