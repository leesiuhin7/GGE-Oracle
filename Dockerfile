FROM python:3.11-slim AS builder

RUN apt-get update && apt-get install -y build-essential curl

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"

RUN pip install maturin

WORKDIR /usr/src/app

# Dummy build to cache dependencies
COPY pyproject.toml .
COPY ./rust/Cargo.toml rust/Cargo.toml
RUN maturin build || true

# Build wheel
COPY . .
RUN maturin build -r -o ./wheels/

FROM python:3.11-slim

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y libjemalloc2
ENV LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libjemalloc.so.2

COPY --from=builder /usr/src/app/wheels wheels

RUN pip install ./wheels/*.whl

CMD ["python", "-m", "gge_oracle.main"]