FROM python:3.11-slim

WORKDIR /usr/src/app

COPY pyproject.toml .
COPY ./src src
RUN pip install --no-cache-dir .

CMD ["python", "-m", "gge_oracle.main"]