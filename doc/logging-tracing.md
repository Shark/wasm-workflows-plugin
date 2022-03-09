# Logging

The application logs to standard output.

The default log level is `INFO`. You can increase it to `DEBUG` by specifying the flag `--debug` or setting the environment variable `DEBUG=1`.

# Tracing

The plugin supports sending [OpenTelemetry](https://opentelemetry.io) traces, for example to a [Jaeger](https://www.jaegertracing.io) instance. 

Telemetry transmission over UDP is enabled by setting the environment variable `OTEL_ENABLE=1`. The integration can be configured as documented in the [OpenTelemetry SDK specification](https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/sdk-environment-variables.md).

A minimal Jaeger setup can be started with Docker:

```shell
docker run --name jaeger \
  -p6831:6831/udp \
  -p6832:6832/udp \
  -p16686:16686 \
  jaegertracing/all-in-one:latest
```

Open [http://localhost:16686](http://localhost:16686) and search for traces by the _service_ `wasm-workflows-plugin`.
